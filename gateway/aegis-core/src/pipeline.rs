//! High-level pipeline orchestration: ingest → dedup → (HEC sink | stderr sink).
//!
//! The caller (typically `aegis-daemon`) owns the `Queue` so that the same
//! queue handle can also be passed to the MCP server for tools like
//! `reset` and (future) `replay_raw`. If `queue` is `None` here, events
//! are pretty-printed to stderr instead (demo mode without Splunk).

use crate::config::AegisConfig;
use crate::control::Control;
use crate::dedup::{self, DedupParams};
use crate::event::{IngestLine, ProcessedEvent};
use crate::hec::HecClient;
use crate::hec_sink::{self, HecSinkConfig};
use crate::queue::Queue;
use crate::sidecar::SidecarClient;
use crate::{ingest, self_metrics, sink};
use anyhow::Context;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// Run the ingest pipeline. Returns when the dedup or sink task exits
/// (which normally means somebody asked it to).
///
/// `queue` is required to enable the HEC sink. Pass `None` to fall back
/// to the stderr sink (demo mode).
pub async fn run(cfg: AegisConfig, control: Control, queue: Option<Queue>) -> anyhow::Result<()> {
    let (lines_tx, lines_rx) = mpsc::channel::<IngestLine>(4096);
    let (out_tx, out_rx) = mpsc::channel::<ProcessedEvent>(4096);

    let mut tasks: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();

    if let Some(addr) = cfg.ingest.tcp_listen.as_deref() {
        let parsed = addr
            .parse()
            .with_context(|| format!("parse tcp_listen {addr:?}"))?;
        let tx = lines_tx.clone();
        tasks.push(tokio::spawn(ingest::run_tcp(parsed, tx)));
    }
    if let Some(addr) = cfg.ingest.udp_listen.as_deref() {
        let parsed = addr
            .parse()
            .with_context(|| format!("parse udp_listen {addr:?}"))?;
        let tx = lines_tx.clone();
        tasks.push(tokio::spawn(ingest::run_udp(parsed, tx)));
    }
    drop(lines_tx);

    let sidecar = if cfg.sidecar.enabled {
        match SidecarClient::new(cfg.sidecar.url.clone(), Duration::from_secs(6)) {
            Ok(client) => {
                info!(url = %cfg.sidecar.url, "AI sidecar enabled");
                Some(client)
            }
            Err(e) => {
                warn!(error = %e, "failed to build sidecar client; continuing without AI");
                None
            }
        }
    } else {
        None
    };

    let dedup_params = DedupParams {
        window_secs: cfg.dedup.window_secs,
        max_open_signatures: cfg.dedup.max_open_signatures,
        sidecar,
        summarize_routine: cfg.summary.enabled,
        summary_flush_secs: cfg.summary.flush_secs,
    };
    if cfg.summary.enabled {
        info!(
            flush_secs = cfg.summary.flush_secs,
            "routine-traffic summarization enabled"
        );
    }
    let dedup_task = tokio::spawn(dedup::run(dedup_params, control.clone(), lines_rx, out_tx));

    let sink_task: JoinHandle<anyhow::Result<()>> = match (cfg.hec.clone(), queue) {
        (Some(hec_cfg), Some(queue)) => {
            info!(endpoint = %hec_cfg.endpoint, "HEC configured; using queue-backed sink");
            let hec = HecClient::new(hec_cfg.clone())?;
            let sink_cfg = HecSinkConfig {
                host: hec_cfg.host.clone().unwrap_or_else(default_host),
                index: hec_cfg.index.clone(),
                ..Default::default()
            };

            let drain_handle = tokio::spawn(hec_sink::run_drain(
                sink_cfg.clone(),
                hec.clone(),
                queue.clone(),
                control.clone(),
            ));
            tasks.push(drain_handle);

            let metrics_handle = tokio::spawn(self_metrics::run(
                cfg.self_metrics.clone(),
                hec.clone(),
                control.clone(),
                sink_cfg.host.clone(),
                sink_cfg.index.clone(),
            ));
            tasks.push(metrics_handle);

            tokio::spawn(hec_sink::run_enqueue(queue, control.clone(), out_rx))
        }
        _ => {
            info!("HEC unavailable or no queue; using stderr sink (demo mode)");
            tokio::spawn(sink::run_stderr(out_rx))
        }
    };

    info!(
        window_secs = cfg.dedup.window_secs,
        max_open = cfg.dedup.max_open_signatures,
        "pipeline running"
    );

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("ctrl-c received, shutting down pipeline");
        }
        res = dedup_task => report("dedup", res),
        res = sink_task => report("sink", res),
    }

    for h in tasks {
        h.abort();
    }
    let _ = tokio::time::timeout(Duration::from_millis(200), async {
        tokio::task::yield_now().await
    })
    .await;
    Ok(())
}

fn report(name: &str, res: Result<anyhow::Result<()>, tokio::task::JoinError>) {
    match res {
        Ok(Ok(())) => warn!(task = name, "task exited"),
        Ok(Err(e)) => warn!(task = name, error = %e, "task failed"),
        Err(e) => warn!(task = name, error = %e, "task panicked"),
    }
}

fn default_host() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "aegis-edge".to_string())
}
