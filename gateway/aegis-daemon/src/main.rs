//! Aegis daemon entrypoint.
//!
//! Wires the data plane (`aegis-core`) and the MCP control plane
//! (`aegis-mcp`) together with a *shared* `Control` and `Queue`, so that
//! an external MCP client (Cursor, Claude Desktop) can inspect and
//! mutate the live state of the running ingest pipeline.
//!
//! Modes:
//!   * default              — pipeline + MCP HTTP server (recommended)
//!   * `--no-mcp`           — pipeline only
//!   * `--mcp-only`         — stdio MCP server only (for clients that
//!                            spawn the daemon as a subprocess)
//!   * `--mcp-http-only`    — MCP HTTP server only (useful when an
//!                            external process is feeding HEC)
//!   * `--check-hec`        — send one ping event and exit

use aegis_core::hec::{HecClient, HecEvent};
use aegis_core::{pipeline, AegisConfig, Control, Queue};
use aegis_mcp::AegisMcp;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{error, info, warn};

#[derive(Debug, Parser)]
#[command(name = "aegis-daemon", version, about)]
struct Args {
    /// Path to the TOML configuration file.
    #[arg(short, long, default_value = "configs/aegis.toml")]
    config: PathBuf,

    /// Run only the stdio MCP server (skip pipeline). Used by MCP clients
    /// that spawn the daemon as a subprocess.
    #[arg(long, conflicts_with_all = ["no_mcp", "pipeline_only", "mcp_http_only", "check_hec"])]
    mcp_only: bool,

    /// Run only the MCP HTTP server (skip pipeline).
    #[arg(long, conflicts_with_all = ["no_mcp", "pipeline_only", "mcp_only", "check_hec"])]
    mcp_http_only: bool,

    /// Run only the ingest pipeline (skip MCP servers).
    #[arg(long, conflicts_with_all = ["mcp_only", "mcp_http_only", "check_hec"])]
    no_mcp: bool,

    /// Deprecated alias for `--no-mcp`. Kept for older docs.
    #[arg(long, hide = true, conflicts_with_all = ["mcp_only", "mcp_http_only", "check_hec"])]
    pipeline_only: bool,

    /// Send a single ping event to HEC and exit. Useful for verifying
    /// HEC endpoint, token, and TLS settings before running the pipeline.
    #[arg(long, conflicts_with_all = ["mcp_only", "mcp_http_only", "no_mcp", "pipeline_only"])]
    check_hec: bool,

    /// Override the MCP HTTP listen address from the config file.
    #[arg(long, value_name = "ADDR")]
    mcp_listen: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let args = Args::parse();

    let cfg = AegisConfig::load_or_default(&args.config)?;
    let control = Control::new();

    info!(
        config = %args.config.display(),
        mcp_only = args.mcp_only,
        mcp_http_only = args.mcp_http_only,
        no_mcp = args.no_mcp || args.pipeline_only,
        "starting aegis-daemon"
    );

    if args.check_hec {
        return check_hec(&cfg).await;
    }

    if args.mcp_only {
        // No shared state: this mode is for MCP clients that spawn us as a
        // subprocess. Useful for smoke-testing the tool surface.
        return AegisMcp::new(control).serve_stdio().await;
    }

    // The queue is needed by both the HEC sink (inside the pipeline) and
    // the MCP `reset` tool. Construct it here so both halves share the
    // same handle.
    let queue = build_queue_if_possible(&cfg);

    if args.mcp_http_only {
        let addr = resolve_mcp_listen(&cfg, args.mcp_listen.as_deref())?;
        let mut mcp = AegisMcp::new(control.clone());
        if let Some(q) = queue {
            mcp = mcp.with_queue(q);
        }
        return mcp.serve_http(addr).await;
    }

    let pipeline_task = tokio::spawn(pipeline::run(cfg.clone(), control.clone(), queue.clone()));

    let mcp_task_opt: Option<tokio::task::JoinHandle<anyhow::Result<()>>> =
        if args.no_mcp || args.pipeline_only {
            None
        } else {
            match resolve_mcp_listen(&cfg, args.mcp_listen.as_deref()).ok() {
                Some(addr) => {
                    let mut mcp = AegisMcp::new(control.clone());
                    if let Some(q) = queue {
                        mcp = mcp.with_queue(q);
                    }
                    Some(tokio::spawn(mcp.serve_http(addr)))
                }
                None => {
                    warn!("mcp.http_listen not set; MCP HTTP server disabled");
                    None
                }
            }
        };

    let (winner, outcome) = match mcp_task_opt {
        Some(mcp_task) => tokio::select! {
            res = pipeline_task => ("pipeline", res),
            res = mcp_task => ("mcp", res),
        },
        None => ("pipeline", pipeline_task.await),
    };

    match outcome {
        Ok(Ok(())) => info!(task = winner, "exited cleanly"),
        Ok(Err(e)) => error!(task = winner, error = %e, "failed"),
        Err(e) => error!(task = winner, error = %e, "panicked"),
    }
    Ok(())
}

fn build_queue_if_possible(cfg: &AegisConfig) -> Option<Queue> {
    if cfg.hec.is_none() {
        return None;
    }
    match Queue::open(&cfg.queue.path, cfg.queue.max_disk_bytes) {
        Ok(q) => Some(q),
        Err(e) => {
            error!(error = %e, "failed to open queue; HEC sink will be disabled");
            None
        }
    }
}

fn resolve_mcp_listen(cfg: &AegisConfig, cli_override: Option<&str>) -> anyhow::Result<SocketAddr> {
    let raw = cli_override
        .map(str::to_string)
        .or_else(|| cfg.mcp.http_listen.clone())
        .ok_or_else(|| anyhow::anyhow!("no mcp.http_listen configured (and no --mcp-listen given)"))?;
    raw.parse()
        .map_err(|e| anyhow::anyhow!("parse mcp listen {raw:?}: {e}"))
}

async fn check_hec(cfg: &AegisConfig) -> anyhow::Result<()> {
    let Some(hec_cfg) = cfg.hec.clone() else {
        error!("no [hec] section in config; nothing to check");
        anyhow::bail!("hec not configured");
    };
    info!(endpoint = %hec_cfg.endpoint, verify_tls = hec_cfg.verify_tls, "sending HEC ping");
    let client = HecClient::new(hec_cfg.clone())?;
    let ping = HecEvent {
        time: now_secs_f64(),
        host: hec_cfg.host.clone().unwrap_or_else(|| "aegis-edge".into()),
        source: "aegis:diagnostic".into(),
        sourcetype: "aegis:diagnostic".into(),
        index: hec_cfg.index.clone(),
        event: serde_json::json!({
            "kind": "startup_ping",
            "message": "aegis-daemon hec check successful",
        }),
    };
    client.send(std::slice::from_ref(&ping)).await?;
    info!("HEC ping accepted; check your Splunk for sourcetype=aegis:diagnostic");
    Ok(())
}

fn now_secs_f64() -> f64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    // Logs go to stderr so stdout stays clean for any future stdio MCP
    // use case.
    let env = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(env)
        .with_writer(std::io::stderr)
        .with_target(false)
        .compact()
        .init();
}
