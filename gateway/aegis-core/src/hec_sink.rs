//! Queue-backed HEC sink.
//!
//! Pipeline shape:
//!   * `enqueue_loop` reads `ProcessedEvent`s from the dedup channel and
//!     persists them to the SQLite queue. This always succeeds (within disk
//!     budget) regardless of HEC health.
//!   * `drain_loop` periodically peeks the highest-priority batch, sends it
//!     to HEC, acks on success. On failure, marks the gateway offline and
//!     backs off exponentially.
//!
//! When HEC is unreachable, events keep landing in the queue ordered by
//! priority. When HEC recovers, anomalies drain first.

use crate::control::Control;
use crate::event::ProcessedEvent;
use crate::hec::{HecClient, HecEvent};
use crate::queue::Queue;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};

#[derive(Clone, Debug)]
pub struct HecSinkConfig {
    pub batch_size: usize,
    pub drain_interval: Duration,
    pub max_backoff: Duration,
    pub host: String,
    pub index: Option<String>,
}

impl Default for HecSinkConfig {
    fn default() -> Self {
        Self {
            batch_size: 200,
            drain_interval: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
            host: hostname_or_default(),
            index: None,
        }
    }
}

/// Run the enqueue half of the sink. Returns when `rx` closes.
pub async fn run_enqueue(
    queue: Queue,
    control: Control,
    mut rx: mpsc::Receiver<ProcessedEvent>,
) -> anyhow::Result<()> {
    info!("hec sink enqueue loop running");
    while let Some(ev) = rx.recv().await {
        if let Err(e) = queue.enqueue(&ev).await {
            warn!(error = %e, "queue enqueue failed; dropping event");
            continue;
        }
        if let Ok(depth) = queue.depth().await {
            control.set_queue_depth(depth);
        }
    }
    Ok(())
}

/// Run the drain half of the sink. Loops until cancelled by abort.
pub async fn run_drain(
    cfg: HecSinkConfig,
    hec: HecClient,
    queue: Queue,
    control: Control,
) -> anyhow::Result<()> {
    info!(
        batch_size = cfg.batch_size,
        host = %cfg.host,
        "hec sink drain loop running"
    );
    let mut backoff = cfg.drain_interval;

    loop {
        let batch = match queue.peek_batch(cfg.batch_size).await {
            Ok(b) => b,
            Err(e) => {
                warn!(error = %e, "queue peek failed");
                sleep(backoff).await;
                continue;
            }
        };

        if batch.is_empty() {
            sleep(cfg.drain_interval).await;
            continue;
        }

        let hec_events: Vec<HecEvent> = batch
            .iter()
            .map(|item| item.event.to_hec_event(&cfg.host, cfg.index.as_deref()))
            .collect();

        match hec.send(&hec_events).await {
            Ok(()) => {
                let ids: Vec<i64> = batch.iter().map(|i| i.id).collect();
                if let Err(e) = queue.ack(&ids).await {
                    warn!(error = %e, "queue ack failed; events may be redelivered");
                }
                control.set_online(true);
                if let Ok(depth) = queue.depth().await {
                    control.set_queue_depth(depth);
                }
                backoff = cfg.drain_interval;
                debug!(sent = ids.len(), "hec batch delivered");
            }
            Err(e) => {
                control.set_online(false);
                warn!(error = %e, batch = batch.len(), "hec send failed; backing off");
                sleep(backoff).await;
                backoff = (backoff * 2).min(cfg.max_backoff);
            }
        }
    }
}

fn hostname_or_default() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "aegis-edge".to_string())
}
