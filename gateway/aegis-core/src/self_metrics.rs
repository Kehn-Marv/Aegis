//! Periodic self-metrics emitter.
//!
//! Snapshots `Control` on a timer and pushes a JSON event to HEC under the
//! dedicated `aegis:selfmetric` sourcetype. Drives the Splunk dashboard's
//! live KPIs (dedup savings, queue depth, health state, incidents
//! remembered) and proves the agent isn't introducing latency.

use crate::config::SelfMetricsConfig;
use crate::control::Control;
use crate::hec::{HecClient, HecEvent};
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, warn};

pub async fn run(
    cfg: SelfMetricsConfig,
    hec: HecClient,
    control: Control,
    host: String,
    index: Option<String>,
) -> anyhow::Result<()> {
    let mut tick = interval(Duration::from_secs(cfg.flush_secs.max(1)));
    // Skip the first immediate tick so we don't emit before the pipeline
    // has any data to report on.
    tick.tick().await;
    loop {
        tick.tick().await;
        let snap = control.snapshot();
        let event = HecEvent {
            time: now_secs_f64(),
            host: host.clone(),
            source: cfg.source.clone(),
            sourcetype: cfg.sourcetype.clone(),
            index: index.clone(),
            event: serde_json::to_value(&snap).unwrap_or(serde_json::Value::Null),
        };
        match hec.send(std::slice::from_ref(&event)).await {
            Ok(()) => debug!(?snap, "self-metric snapshot emitted"),
            Err(e) => warn!(error = %e, "self-metric emit failed"),
        }
    }
}

fn now_secs_f64() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
