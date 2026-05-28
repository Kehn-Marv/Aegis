//! High-level pipeline orchestration.
//!
//! Flow (left to right):
//!
//! ```text
//!   ingest ──▶ dedup ──▶ causal ──▶ silence ──▶ decision ──▶ sink
//!                            │                       │
//!                            └──────memory store◀────┘
//! ```
//!
//! Each stage consumes one channel and produces another. Every event
//! flows through every stage; the new stages (causal, silence, decision)
//! pass everything through unchanged and additionally emit
//! `CausalChain` / `ServiceSilent` / `DecisionCard` / `IncidentMemory`
//! when their conditions trigger.

use crate::causal::CausalParams;
use crate::config::AegisConfig;
use crate::control::Control;
use crate::decision::DecisionParams;
use crate::dedup::{self, DedupParams};
use crate::event::{IngestLine, ProcessedEvent};
use crate::hec::HecClient;
use crate::hec_sink::{self, HecSinkConfig};
use crate::incident_memory::Store as IncidentStore;
use crate::queue::Queue;
use crate::service_catalog::ServiceCatalog;
use crate::sidecar::SidecarClient;
use crate::silence::SilenceParams;
use crate::{causal, decision, ingest, self_metrics, silence, sink};
use anyhow::Context;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// Run the ingest pipeline. Returns when the dedup or sink task exits
/// (which normally means somebody asked it to).
///
/// `queue` is required to enable the HEC sink. Pass `None` to fall back
/// to the stderr sink (demo mode).
pub async fn run(cfg: AegisConfig, control: Control, queue: Option<Queue>) -> anyhow::Result<()> {
    let (lines_tx, lines_rx) = mpsc::channel::<IngestLine>(4096);
    let (dedup_out_tx, dedup_out_rx) = mpsc::channel::<ProcessedEvent>(4096);
    let (causal_out_tx, causal_out_rx) = mpsc::channel::<ProcessedEvent>(4096);
    let (silence_out_tx, silence_out_rx) = mpsc::channel::<ProcessedEvent>(4096);
    let (final_tx, final_rx) = mpsc::channel::<ProcessedEvent>(4096);

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
        source_to_service: cfg.source_to_service.clone(),
    };
    if cfg.summary.enabled {
        info!(
            flush_secs = cfg.summary.flush_secs,
            "routine-traffic summarization enabled"
        );
    }
    let dedup_task = tokio::spawn(dedup::run(
        dedup_params,
        control.clone(),
        lines_rx,
        dedup_out_tx,
    ));

    let causal_params = CausalParams {
        window_secs: cfg.causal.window_secs,
        min_services: cfg.causal.min_services,
        cooldown_secs: cfg.causal.cooldown_secs,
        ..Default::default()
    };
    info!(
        window_secs = causal_params.window_secs,
        min_services = causal_params.min_services,
        "causal chain detector running"
    );
    tasks.push(tokio::spawn(causal::run(causal_params, dedup_out_rx, causal_out_tx)));

    // Silence detector — runs before the decision stage so a silent service
    // also goes into the latest-decision slot.
    if cfg.silence.enabled {
        let silence_params = SilenceParams {
            silence_secs: cfg.silence.silence_secs,
            sweep_secs: cfg.silence.sweep_secs,
        };
        info!(
            silence_secs = silence_params.silence_secs,
            "silent-service detector running"
        );
        tasks.push(tokio::spawn(silence::run(
            silence_params,
            causal_out_rx,
            silence_out_tx,
        )));
    } else {
        // Pass-through: still need to forward events.
        info!("silent-service detector disabled");
        let tx = silence_out_tx.clone();
        let mut rx = causal_out_rx;
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                if tx.send(ev).await.is_err() {
                    break;
                }
            }
            Ok(())
        }));
    }

    // Incident memory store + decision engine.
    let store = IncidentStore::open(&cfg.memory.path)
        .with_context(|| format!("open incident memory at {}", cfg.memory.path))?;
    control.set_incidents_remembered(store.count().unwrap_or(0));
    info!(
        path = %cfg.memory.path,
        existing = control.snapshot().incidents_remembered,
        "incident memory store ready"
    );

    let catalog = ServiceCatalog::from_map(cfg.services.clone());
    if !catalog.is_empty() {
        info!(
            services = catalog.len(),
            "service catalogue loaded (business-impact text attached to decision cards)"
        );
    }
    let decision_params = DecisionParams {
        max_similar_incidents: cfg.memory.top_matches,
        min_similarity: cfg.memory.min_similarity,
        idle_to_green_secs: cfg.decision.idle_to_green_secs,
    };
    let (card_tx, card_rx) = watch::channel::<Option<ProcessedEvent>>(None);
    tasks.push(tokio::spawn(decision::run(
        decision_params,
        store.clone(),
        catalog,
        silence_out_rx,
        final_tx,
        card_tx,
    )));

    // Listener: keep `Control.latest_decision` + `Control.state` in sync
    // with the latest card the decision engine emits.
    {
        let control = control.clone();
        let store = store.clone();
        tasks.push(tokio::spawn(async move {
            let mut rx = card_rx;
            loop {
                if rx.changed().await.is_err() {
                    break;
                }
                let value = rx.borrow().clone();
                if let Some(ProcessedEvent::DecisionCard { state, .. }) = &value {
                    control.set_state(*state);
                }
                control.set_latest_decision(value);
                control.set_incidents_remembered(store.count().unwrap_or(0));
            }
            Ok(())
        }));
    }

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

            tokio::spawn(hec_sink::run_enqueue(queue, control.clone(), final_rx))
        }
        _ => {
            info!("HEC unavailable or no queue; using stderr sink (demo mode)");
            tokio::spawn(sink::run_stderr(final_rx))
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

/// Construct a queue-backed incident store reference for the MCP layer.
/// Exposed here so `aegis-daemon` and `aegis-mcp` can share a single
/// open handle.
pub fn open_incident_store(path: &str) -> anyhow::Result<IncidentStore> {
    IncidentStore::open(path).context("open incident memory store")
}
