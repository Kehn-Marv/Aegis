//! MCP server exposing the Aegis control surface.
//!
//! Tools:
//!   * `status`     — return the live `GatewayStatus` snapshot as JSON
//!   * `reset`      — clear in-memory counters and (if attached) the queue
//!   * `diagnostic` — enable verbose tracing at the edge for N seconds
//!   * `override`   — disable compression for N seconds (raw passthrough)
//!   * `replay_raw` — re-emit buffered raw events for a given time window
//!                   (stub — see TODO inside)
//!
//! Two transports are supported:
//!   * `serve_stdio()` — for MCP clients that spawn the daemon as a child
//!     process. The resulting server has its *own* `Control` and is useful
//!     mostly for smoke-testing the tool surface in isolation.
//!   * `serve_http(addr)` — bind a streamable-HTTP MCP server at `addr` so
//!     remote MCP clients (Cursor, Claude Desktop, custom orchestrators)
//!     can control the *running* daemon. Because `Control` and `Queue` are
//!     both Arc-inside, every accepted session shares the same live state
//!     as the ingest pipeline.

use std::net::SocketAddr;
use std::sync::Arc;

use aegis_core::{Control, GatewayStatus, Queue};
use anyhow::Context;
use axum::extract::State;
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
use tracing::info;

/// MCP server handle. Cheap to clone — `Control` and `Queue` are both
/// Arc-backed, so cloning produces a new struct that points at the same
/// live gateway state. This is exactly what we want for the streamable
/// HTTP server's per-session factory.
#[derive(Clone)]
pub struct AegisMcp {
    control: Control,
    queue: Option<Queue>,
}

impl AegisMcp {
    pub fn new(control: Control) -> Self {
        Self {
            control,
            queue: None,
        }
    }

    /// Attach a live queue handle so `reset` and `replay_raw` can mutate
    /// persistent state in addition to the in-memory counters.
    pub fn with_queue(mut self, queue: Queue) -> Self {
        self.queue = Some(queue);
        self
    }

    /// Serve over stdio. The MCP client (e.g. Cursor) spawns the daemon
    /// as a subprocess; this is the simplest transport but does not share
    /// state with any other Aegis process.
    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let service = self.serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        Ok(())
    }

    /// Bind a streamable-HTTP MCP server **plus** a small REST API that
    /// the control-panel UI consumes. Endpoints:
    ///
    ///   * `POST /mcp`        — MCP protocol (Cursor, Claude Desktop, ...)
    ///   * `GET  /api/status` — JSON snapshot of the live `Control`
    ///   * `POST /api/command`— `{ command: "...", seconds?: u64 }`
    ///   * `GET  /api/health` — liveness probe
    ///
    /// CORS is permissive — the daemon binds to `127.0.0.1` by default
    /// so the only origins able to reach it are local browsers. If you
    /// expose the daemon beyond localhost, put it behind an auth proxy.
    pub async fn serve_http(self, addr: SocketAddr) -> anyhow::Result<()> {
        let api_state = ApiState {
            control: self.control.clone(),
            queue: self.queue.clone(),
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
            .route("/command", post(api_command))
            .route("/health", get(api_health))
            .with_state(api_state);

        let app = Router::new()
            .nest("/api", api_router)
            .nest_service("/mcp", mcp_service)
            .layer(CorsLayer::permissive());

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

/// State shared by the REST API handlers.
#[derive(Clone)]
struct ApiState {
    control: Control,
    queue: Option<Queue>,
}

async fn api_health() -> &'static str {
    "ok"
}

async fn api_status(State(state): State<ApiState>) -> Json<GatewayStatus> {
    Json(state.control.snapshot())
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

#[tool_router]
impl AegisMcp {
    #[tool(description = "Return the current Aegis gateway status snapshot as JSON.")]
    fn status(&self) -> String {
        serde_json::to_string(&self.control.snapshot()).unwrap_or_else(|e| {
            format!("{{\"error\":\"failed to serialize status: {}\"}}", e)
        })
    }

    #[tool(description = "Reset the gateway: clear the priority queue, dedup table, and counters.")]
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

    #[tool(description = "Enable verbose diagnostic tracing at the edge for N seconds.")]
    fn diagnostic(&self, Parameters(DurationParams { seconds }): Parameters<DurationParams>) -> String {
        self.control.enable_diagnostic(seconds);
        format!("diagnostic: enabled for {} seconds", seconds)
    }

    #[tool(description = "Disable compression and stream raw logs to HEC for N seconds.")]
    fn r#override(&self, Parameters(DurationParams { seconds }): Parameters<DurationParams>) -> String {
        self.control.enable_override(seconds);
        format!("override: raw passthrough enabled for {} seconds", seconds)
    }

    #[tool(description = "Re-emit buffered raw events for the given unix-time window. (Not yet wired to a persistent history table; the current queue acks events after send. See docs/mcp.md.)")]
    fn replay_raw(&self, Parameters(ReplayParams { from, to }): Parameters<ReplayParams>) -> String {
        let queued = self.queue.is_some();
        format!(
            "replay_raw: stub. requested window {from}..{to}, queue_attached={queued}. \
             Will be wired in Phase 1c when we add a separate history table."
        )
    }
}

#[tool_handler(
    name = "aegis-mcp",
    version = "0.1.0",
    instructions = "Aegis Edge-Telemetry Gateway control surface. Use `status` to inspect live gateway health (queue depth, dedup ratio, online flag). Use `override(seconds)` during an incident to stream uncompressed raw logs to Splunk for a bounded window. `diagnostic(seconds)` enables verbose tracing at the edge. `reset` clears the priority queue and in-memory dedup counters."
)]
impl ServerHandler for AegisMcp {}
