//! MCP server + REST control plane for the Aegis daemon.
//!
//! Endpoints:
//!
//! | path                                | what it does                                   |
//! |-------------------------------------|------------------------------------------------|
//! | `POST /mcp`                         | MCP JSON-RPC (Cursor, Claude Desktop, …)       |
//! | `GET  /api/status`                  | live gateway snapshot (health, KPIs, decision) |
//! | `GET  /api/decision`                | the current decision card (`null` if green)    |
//! | `POST /api/decision/ack`            | engineer clicks "I'm on it"                    |
//! | `GET  /api/incidents`               | recent fingerprints from the memory store       |
//! | `GET  /api/incidents/{id}`          | one fingerprint by id                           |
//! | `POST /api/incidents/{id}/resolve`  | submit the resolution card                      |
//! | `POST /api/command`                 | legacy command shim (used by UI + smoke tests) |
//! | `GET  /api/health`                  | liveness probe                                  |
//!
//! MCP tools published:
//!
//! | tool                  | description                                              |
//! |-----------------------|----------------------------------------------------------|
//! | `status`              | live snapshot (mirrors `/api/status`)                    |
//! | `latest_decision`     | the current decision card                                |
//! | `recent_incidents`    | top-N most recent fingerprints                           |
//! | `resolve_incident`    | attach a cause+fix resolution card                       |
//! | `acknowledge`         | mark the current decision as "I'm on it"                 |
//! | `reset`               | clear queue + dedup counters                             |
//! | `diagnostic` / `override` / `replay_raw` — unchanged from v0.1.        |

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use aegis_core::{
    incident_memory::ResolutionCard, Control, Fingerprint, GatewayStatus, IncidentStore,
    ProcessedEvent, Queue,
};
use anyhow::Context;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpService;
use rmcp::transport::StreamableHttpServerConfig;
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

/// MCP server handle. Cheap to clone — `Control`, `Queue`, and `IncidentStore`
/// are all `Arc`-backed, so cloning produces a new struct that points at the
/// same live state.
#[derive(Clone)]
pub struct AegisMcp {
    control: Control,
    queue: Option<Queue>,
    store: Option<IncidentStore>,
}

impl AegisMcp {
    pub fn new(control: Control) -> Self {
        Self {
            control,
            queue: None,
            store: None,
        }
    }

    pub fn with_queue(mut self, queue: Queue) -> Self {
        self.queue = Some(queue);
        self
    }

    pub fn with_store(mut self, store: IncidentStore) -> Self {
        self.store = Some(store);
        self
    }

    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let service = self.serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        Ok(())
    }

    pub async fn serve_http(self, addr: SocketAddr) -> anyhow::Result<()> {
        let api_state = ApiState {
            control: self.control.clone(),
            queue: self.queue.clone(),
            store: self.store.clone(),
        };
        let template = self;
        let factory = move || Ok::<AegisMcp, std::io::Error>(template.clone());
        let session_manager = Arc::new(LocalSessionManager::default());
        let mcp_service = StreamableHttpService::new(
            factory,
            session_manager,
            StreamableHttpServerConfig::default(),
        );

        let api_router = Router::new()
            .route("/status", get(api_status))
            .route("/decision", get(api_decision))
            .route("/decision/ack", post(api_decision_ack))
            .route("/incidents", get(api_incidents))
            .route("/incidents/{id}", get(api_incident_get))
            .route("/incidents/{id}/resolve", post(api_incident_resolve))
            .route("/command", post(api_command))
            .route("/health", get(api_health))
            .with_state(api_state);

        let mut app = Router::new()
            .nest("/api", api_router)
            .nest_service("/mcp", mcp_service);

        // If a built control-panel UI is present, serve it from `/` with an
        // SPA fallback to index.html. This means the daemon ships its own UI
        // (no separate static server needed) both locally and in the
        // container. Set AEGIS_UI_DIR to override the location.
        if let Some(dir) = resolve_ui_dir() {
            let index = dir.join("index.html");
            info!(ui = %dir.display(), "serving control panel UI at {addr}/");
            let static_svc = ServeDir::new(&dir).fallback(ServeFile::new(index));
            app = app.fallback_service(static_svc);
        }

        let app = app.layer(CorsLayer::permissive());

        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("bind aegis http listener at {addr}"))?;
        info!(%addr, "MCP HTTP listening at {addr}/mcp");
        info!(%addr, "Control API at {addr}/api/status");
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await
            .context("aegis http serve")?;
        Ok(())
    }
}

/// Locate a built UI directory, if one exists. Checks `AEGIS_UI_DIR` first,
/// then a couple of conventional locations (local dev build, container path).
fn resolve_ui_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("AEGIS_UI_DIR") {
        let p = PathBuf::from(dir);
        if p.join("index.html").is_file() {
            return Some(p);
        }
    }
    for cand in ["ui/dist", "/app/ui"] {
        let p = PathBuf::from(cand);
        if p.join("index.html").is_file() {
            return Some(p);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// REST handlers
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ApiState {
    control: Control,
    queue: Option<Queue>,
    store: Option<IncidentStore>,
}

async fn api_health() -> &'static str {
    "ok"
}

async fn api_status(State(state): State<ApiState>) -> Json<GatewayStatus> {
    Json(state.control.snapshot())
}

async fn api_decision(State(state): State<ApiState>) -> Json<Option<ProcessedEvent>> {
    Json(state.control.latest_decision())
}

#[derive(Debug, Default, Deserialize)]
struct AckBody {
    /// Identifier for the operator who took ownership (free-form: name, email,
    /// or a Slack handle). Optional, defaults to `"unknown"`.
    #[serde(default)]
    actor: Option<String>,
}

#[derive(Debug, Serialize)]
struct AckResponse {
    ok: bool,
    message: String,
    decision_id: Option<String>,
    actor: String,
}

async fn api_decision_ack(
    State(state): State<ApiState>,
    Json(body): Json<AckBody>,
) -> Json<AckResponse> {
    let decision = state.control.latest_decision();
    let decision_id = decision.as_ref().and_then(|ev| match ev {
        ProcessedEvent::DecisionCard { decision_id, .. } => Some(decision_id.clone()),
        _ => None,
    });
    let actor = body
        .actor
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();
    info!(?decision_id, %actor, "engineer acknowledged decision card");
    Json(AckResponse {
        ok: decision_id.is_some(),
        message: match decision_id.is_some() {
            true => "decision acknowledged".to_string(),
            false => "no active decision to acknowledge".to_string(),
        },
        decision_id,
        actor,
    })
}

#[derive(Debug, Deserialize)]
struct IncidentsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize)]
struct IncidentsResponse {
    count: usize,
    incidents: Vec<Fingerprint>,
}

async fn api_incidents(
    State(state): State<ApiState>,
    axum::extract::Query(q): axum::extract::Query<IncidentsQuery>,
) -> Json<IncidentsResponse> {
    let Some(store) = state.store.as_ref() else {
        return Json(IncidentsResponse { count: 0, incidents: Vec::new() });
    };
    let incidents = store.recent(q.limit.max(1)).unwrap_or_default();
    Json(IncidentsResponse {
        count: incidents.len(),
        incidents,
    })
}

async fn api_incident_get(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Json<Option<Fingerprint>> {
    let Some(store) = state.store.as_ref() else {
        return Json(None);
    };
    Json(store.get(&id).ok().flatten())
}

#[derive(Debug, Deserialize)]
struct ResolveBody {
    cause: String,
    fix: String,
}

#[derive(Debug, Serialize)]
struct ResolveResponse {
    ok: bool,
    message: String,
    incident: Option<Fingerprint>,
}

async fn api_incident_resolve(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<ResolveBody>,
) -> Json<ResolveResponse> {
    let Some(store) = state.store.as_ref() else {
        return Json(ResolveResponse {
            ok: false,
            message: "incident memory not attached".into(),
            incident: None,
        });
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let card = ResolutionCard {
        cause: body.cause,
        fix: body.fix,
    };
    match store.resolve(&id, card, now) {
        Ok(Some(fp)) => Json(ResolveResponse {
            ok: true,
            message: format!("incident {} resolved", id),
            incident: Some(fp),
        }),
        Ok(None) => Json(ResolveResponse {
            ok: false,
            message: format!("incident {} not found", id),
            incident: None,
        }),
        Err(e) => Json(ResolveResponse {
            ok: false,
            message: format!("resolve failed: {e}"),
            incident: None,
        }),
    }
}

#[derive(Debug, Deserialize)]
struct CommandRequest {
    command: String,
    #[serde(default)]
    seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
struct CommandResponse {
    ok: bool,
    message: String,
}

async fn api_command(
    State(state): State<ApiState>,
    Json(req): Json<CommandRequest>,
) -> Json<CommandResponse> {
    let resp = match req.command.as_str() {
        "reset" => {
            state.control.reset();
            if let Some(q) = state.queue.as_ref() {
                if let Err(e) = q.clear().await {
                    return Json(CommandResponse {
                        ok: false,
                        message: format!("counters cleared, queue clear failed: {e}"),
                    });
                }
                CommandResponse {
                    ok: true,
                    message: "counters and queue cleared".into(),
                }
            } else {
                CommandResponse {
                    ok: true,
                    message: "counters cleared (no queue attached)".into(),
                }
            }
        }
        "override" => {
            let s = req.seconds.unwrap_or(30);
            state.control.enable_override(s);
            CommandResponse {
                ok: true,
                message: format!("override raw passthrough enabled for {s}s"),
            }
        }
        "diagnostic" => {
            let s = req.seconds.unwrap_or(30);
            state.control.enable_diagnostic(s);
            CommandResponse {
                ok: true,
                message: format!("diagnostic tracing enabled for {s}s"),
            }
        }
        "online" => {
            state.control.set_online(true);
            CommandResponse {
                ok: true,
                message: "marked online".into(),
            }
        }
        "offline" => {
            state.control.set_online(false);
            CommandResponse {
                ok: true,
                message: "marked offline".into(),
            }
        }
        other => CommandResponse {
            ok: false,
            message: format!("unknown command: {other}"),
        },
    };
    Json(resp)
}

// ---------------------------------------------------------------------------
// MCP tool surface
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DurationParams {
    /// How many seconds the mode should remain active.
    pub seconds: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReplayParams {
    /// Start of the replay window (unix epoch seconds, inclusive).
    pub from: u64,
    /// End of the replay window (unix epoch seconds, exclusive).
    pub to: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IncidentsParams {
    /// Maximum number of recent incidents to return. Defaults to 10.
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResolveParams {
    /// The incident id from `recent_incidents` or the decision card.
    pub incident_id: String,
    /// 1–2 sentences explaining the actual root cause.
    pub cause: String,
    /// 1–2 sentences explaining what fixed it.
    pub fix: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AcknowledgeParams {
    /// Free-form identifier for who is taking ownership. Optional.
    #[serde(default)]
    pub actor: Option<String>,
}

#[tool_router]
impl AegisMcp {
    #[tool(description = "Live gateway snapshot: dedup ratio, queue depth, current health state (green/orange/red), and the latest decision card.")]
    fn status(&self) -> String {
        serde_json::to_string(&self.control.snapshot())
            .unwrap_or_else(|e| format!("{{\"error\":\"failed to serialize status: {}\"}}", e))
    }

    #[tool(description = "Return the latest decision card the engineer should be looking at. `null` if the gateway is currently green.")]
    fn latest_decision(&self) -> String {
        serde_json::to_string(&self.control.latest_decision())
            .unwrap_or_else(|_| "null".to_string())
    }

    #[tool(description = "List the most recent fingerprinted incidents from Aegis's memory store. Use this to recall what fixed similar incidents in the past.")]
    fn recent_incidents(
        &self,
        Parameters(IncidentsParams { limit }): Parameters<IncidentsParams>,
    ) -> String {
        let Some(store) = &self.store else {
            return "{\"incidents\":[],\"note\":\"memory store not attached\"}".to_string();
        };
        let limit = limit.unwrap_or(10).max(1);
        let incidents = store.recent(limit).unwrap_or_default();
        serde_json::to_string(&serde_json::json!({
            "count": incidents.len(),
            "incidents": incidents,
        }))
        .unwrap_or_else(|_| "{}".to_string())
    }

    #[tool(description = "Attach a cause + fix resolution card to an incident in Aegis's memory. Aegis surfaces this on the next similar incident.")]
    fn resolve_incident(
        &self,
        Parameters(ResolveParams { incident_id, cause, fix }): Parameters<ResolveParams>,
    ) -> String {
        let Some(store) = &self.store else {
            return "memory store not attached".to_string();
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let card = ResolutionCard { cause, fix };
        match store.resolve(&incident_id, card, now) {
            Ok(Some(fp)) => format!(
                "incident {} resolved (in {} min). Aegis will surface this fix on the next similar incident.",
                fp.id,
                fp.resolved_in_minutes.unwrap_or(0)
            ),
            Ok(None) => format!("incident {} not found", incident_id),
            Err(e) => format!("resolve failed: {e}"),
        }
    }

    #[tool(description = "Mark the current decision card as 'I'm on it'. Records the actor and timestamp; does not actuate any production system.")]
    fn acknowledge(
        &self,
        Parameters(AcknowledgeParams { actor }): Parameters<AcknowledgeParams>,
    ) -> String {
        let decision_id = match self.control.latest_decision() {
            Some(ProcessedEvent::DecisionCard { decision_id, .. }) => decision_id,
            _ => return "no active decision to acknowledge".to_string(),
        };
        let actor = actor.unwrap_or_else(|| "unknown".to_string());
        info!(actor = %actor, decision_id = %decision_id, "decision acknowledged via MCP");
        format!("decision {} acknowledged by {}", decision_id, actor)
    }

    #[tool(description = "Reset the gateway: clear the priority queue, dedup table, counters, and the current decision card.")]
    async fn reset(&self) -> String {
        self.control.reset();
        if let Some(q) = self.queue.as_ref() {
            if let Err(e) = q.clear().await {
                return format!("reset: counters cleared, queue clear FAILED: {e}");
            }
            return "reset: counters and queue cleared".to_string();
        }
        "reset: counters cleared (no queue attached)".to_string()
    }

    #[tool(description = "Enable verbose diagnostic tracing at the edge for N seconds. Low risk; auto-releases when the window expires.")]
    fn diagnostic(&self, Parameters(DurationParams { seconds }): Parameters<DurationParams>) -> String {
        self.control.enable_diagnostic(seconds);
        format!("diagnostic: enabled for {} seconds", seconds)
    }

    #[tool(description = "Disable compression and stream raw logs to HEC for N seconds. Use during active investigation; auto-releases when the window expires.")]
    fn r#override(&self, Parameters(DurationParams { seconds }): Parameters<DurationParams>) -> String {
        self.control.enable_override(seconds);
        format!("override: raw passthrough enabled for {} seconds", seconds)
    }

    #[tool(description = "Re-emit buffered raw events for the given unix-time window. (Not yet wired to a persistent history table; the current queue acks events after send. See docs/mcp.md.)")]
    fn replay_raw(&self, Parameters(ReplayParams { from, to }): Parameters<ReplayParams>) -> String {
        let queued = self.queue.is_some();
        format!(
            "replay_raw: stub. requested window {from}..{to}, queue_attached={queued}. \
             Will be wired in a future iteration that adds a separate history table."
        )
    }
}

#[tool_handler(
    name = "aegis-mcp",
    version = "0.2.0",
    instructions = "Aegis edge-telemetry gateway. Aegis quiets repetitive log noise, identifies which service broke first when an incident fans out, and remembers every incident your team has ever resolved so the next similar one comes with a known fix. Tools: `status` for live KPIs and health state (green/orange/red); `latest_decision` for the current decision card; `recent_incidents` to list past fingerprints; `resolve_incident` to attach a cause+fix card; `acknowledge` to mark a decision 'I'm on it'; `reset` to clear state; `diagnostic`/`override` to enable bounded-window tracing modes."
)]
impl ServerHandler for AegisMcp {}
