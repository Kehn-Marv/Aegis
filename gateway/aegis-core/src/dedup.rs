//! Windowed semantic dedup engine.
//!
//! Behaviour:
//!   * First time we see a signature in the open table, emit `FirstOccurrence`
//!     immediately with the full raw line. This guarantees an operator always
//!     has incident context.
//!   * Subsequent occurrences within the window bump the `count`.
//!   * When the window closes (either because nothing has touched the entry
//!     for `window_secs`, or because we've held it for `2 * window_secs`),
//!     if `count > 1` we emit a single `Collapsed` metric event. If
//!     `count == 1` we drop silently — the `FirstOccurrence` already covered
//!     that single line.
//!   * When override-mode is active on `Control`, the dedup step is bypassed:
//!     every line goes out as a `Raw` event.
//!   * When the open table exceeds `max_open_signatures`, the oldest entry
//!     is flushed early to prevent unbounded memory growth.
//!
//! When a sidecar client is configured, the engine asynchronously classifies
//! each *new* signature (one call per unique signature, never per line) and
//! attaches the classification to the eventual `Collapsed` event.
//!
//! Every event also carries the **service name** extracted from the log line
//! (or pinned via config hint). Downstream stages — causal-chain detection,
//! incident memory, silent-service detection — group by service.

use crate::control::Control;
use crate::event::{Classification, IngestLine, ProcessedEvent};
use crate::service;
use crate::sidecar::SidecarClient;
use crate::signature::{self, Signature};
use crate::summary::SummaryTable;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, warn};

#[derive(Debug)]
struct OpenEntry {
    sig: Signature,
    first_seen_mono: Instant,
    first_seen_unix: f64,
    last_seen_mono: Instant,
    last_seen_unix: f64,
    count: u64,
    sample: String,
    source: String,
    service: String,
}

pub struct DedupParams {
    pub window_secs: u64,
    pub max_open_signatures: usize,
    /// Optional AI sidecar. When set, the engine classifies each *new*
    /// signature in the background and attaches the result to the eventual
    /// `Collapsed` event.
    pub sidecar: Option<SidecarClient>,
    /// When true, `Collapsed` events whose classification is `"routine"`
    /// are suppressed and rolled into periodic `Summary` events instead.
    /// Requires `sidecar` to actually have an effect (classification
    /// labels come from the sidecar).
    pub summarize_routine: bool,
    /// How often the per-source `SummaryTable` is drained into `Summary`
    /// events when `summarize_routine` is true.
    pub summary_flush_secs: u64,
    /// Optional hint that maps an ingest source string to a service name.
    /// Lets multi-tenant edges pin a known service identity to each
    /// listening port regardless of what the log content looks like.
    pub source_to_service: HashMap<String, String>,
}

/// Run the dedup engine. Returns when `lines_rx` is closed.
pub async fn run(
    params: DedupParams,
    control: Control,
    mut lines_rx: mpsc::Receiver<IngestLine>,
    out_tx: mpsc::Sender<ProcessedEvent>,
) -> anyhow::Result<()> {
    let mut open: HashMap<Signature, OpenEntry> = HashMap::new();
    let mut classifications: HashMap<Signature, Classification> = HashMap::new();
    let mut summaries = SummaryTable::default();
    // `source → last successfully-inferred service`. Lets multi-line log
    // records (stack traces, tracebacks) inherit their parent service.
    let mut last_service_for_source: HashMap<String, String> = HashMap::new();
    let window = Duration::from_secs(params.window_secs.max(1));
    let max_age = window * 2;
    let summarize = params.summarize_routine;
    let summary_period = Duration::from_secs(params.summary_flush_secs.max(1));
    let mut ticker = interval(Duration::from_millis(500));
    let mut summary_ticker = interval(summary_period);
    summary_ticker.tick().await; // skip the immediate first fire
    let (cls_tx, mut cls_rx) = mpsc::channel::<(Signature, Classification)>(256);
    let source_to_service = params.source_to_service;

    loop {
        tokio::select! {
            biased;

            maybe_line = lines_rx.recv() => {
                let Some(line) = maybe_line else { break };
                control.observe_in(1);

                let service = service::extract_full(
                    &line.text,
                    &line.source,
                    source_to_service.get(&line.source).map(String::as_str),
                    last_service_for_source.get(&line.source).map(String::as_str),
                );
                // Remember the service for this source only when we managed
                // to extract a real one (not when we fell back to the
                // source string itself).
                if service != line.source {
                    last_service_for_source.insert(line.source.clone(), service.clone());
                }

                if control.override_active() {
                    let _ = out_tx.send(ProcessedEvent::Raw {
                        line: line.text,
                        ts: line.ts_unix,
                        source: line.source,
                        service,
                    }).await;
                    control.observe_out(1);
                    continue;
                }

                let sig = signature::compute(&line.text);
                let now = Instant::now();

                if let Some(entry) = open.get_mut(&sig) {
                    entry.last_seen_mono = now;
                    entry.last_seen_unix = line.ts_unix;
                    entry.count += 1;
                } else {
                    if open.len() >= params.max_open_signatures {
                        evict_oldest(
                            &mut open,
                            &control,
                            &out_tx,
                            &mut classifications,
                            &mut summaries,
                            summarize,
                        ).await;
                    }
                    open.insert(sig, OpenEntry {
                        sig,
                        first_seen_mono: now,
                        first_seen_unix: line.ts_unix,
                        last_seen_mono: now,
                        last_seen_unix: line.ts_unix,
                        count: 1,
                        sample: line.text.clone(),
                        source: line.source.clone(),
                        service: service.clone(),
                    });

                    if let Some(sidecar) = params.sidecar.as_ref() {
                        if !classifications.contains_key(&sig) {
                            spawn_classify(sidecar.clone(), sig, line.text.clone(), cls_tx.clone());
                        }
                    }

                    let ev = ProcessedEvent::FirstOccurrence {
                        signature: sig.to_string(),
                        line: line.text,
                        ts: line.ts_unix,
                        source: line.source,
                        service,
                    };
                    if out_tx.send(ev).await.is_err() {
                        warn!("dedup sink closed; shutting down");
                        return Ok(());
                    }
                    control.observe_out(1);
                    control.set_unique_signatures(open.len() as u64);
                }
            }

            Some((sig, cls)) = cls_rx.recv() => {
                classifications.insert(sig, cls);
            }

            _ = ticker.tick() => {
                flush_expired(
                    &mut open,
                    &control,
                    &out_tx,
                    &mut classifications,
                    &mut summaries,
                    summarize,
                    window,
                    max_age,
                ).await;
            }

            _ = summary_ticker.tick() => {
                if summarize {
                    flush_summaries(&mut summaries, &control, &out_tx).await;
                }
            }
        }
    }

    flush_all(&mut open, &control, &out_tx, &mut classifications, &mut summaries, summarize).await;
    Ok(())
}

async fn flush_summaries(
    summaries: &mut SummaryTable,
    control: &Control,
    out_tx: &mpsc::Sender<ProcessedEvent>,
) {
    for ev in summaries.drain_all() {
        if out_tx.send(ev).await.is_ok() {
            control.observe_out(1);
        }
    }
}

fn spawn_classify(
    sidecar: SidecarClient,
    sig: Signature,
    line: String,
    cls_tx: mpsc::Sender<(Signature, Classification)>,
) {
    tokio::spawn(async move {
        match sidecar.classify(&line).await {
            Ok(resp) => {
                let cls = Classification {
                    label: resp.label,
                    confidence: resp.confidence,
                    strategy: resp.strategy,
                };
                let _ = cls_tx.send((sig, cls)).await;
            }
            Err(e) => debug!(error = %e, "sidecar classify failed (soft error)"),
        }
    });
}

#[allow(clippy::too_many_arguments)]
async fn flush_expired(
    open: &mut HashMap<Signature, OpenEntry>,
    control: &Control,
    out_tx: &mpsc::Sender<ProcessedEvent>,
    classifications: &mut HashMap<Signature, Classification>,
    summaries: &mut SummaryTable,
    summarize: bool,
    window: Duration,
    max_age: Duration,
) {
    let now = Instant::now();
    let expired: Vec<Signature> = open
        .iter()
        .filter(|(_, e)| {
            now.duration_since(e.last_seen_mono) >= window
                || now.duration_since(e.first_seen_mono) >= max_age
        })
        .map(|(s, _)| *s)
        .collect();

    for sig in expired {
        if let Some(entry) = open.remove(&sig) {
            let cls = classifications.remove(&sig);
            emit_collapsed(entry, cls, control, out_tx, summaries, summarize).await;
        }
    }
    control.set_unique_signatures(open.len() as u64);
}

async fn evict_oldest(
    open: &mut HashMap<Signature, OpenEntry>,
    control: &Control,
    out_tx: &mpsc::Sender<ProcessedEvent>,
    classifications: &mut HashMap<Signature, Classification>,
    summaries: &mut SummaryTable,
    summarize: bool,
) {
    if let Some((&sig, _)) = open.iter().min_by_key(|(_, e)| e.first_seen_mono) {
        if let Some(entry) = open.remove(&sig) {
            debug!(signature = %sig, "evicting oldest open signature");
            let cls = classifications.remove(&sig);
            emit_collapsed(entry, cls, control, out_tx, summaries, summarize).await;
        }
    }
}

async fn flush_all(
    open: &mut HashMap<Signature, OpenEntry>,
    control: &Control,
    out_tx: &mpsc::Sender<ProcessedEvent>,
    classifications: &mut HashMap<Signature, Classification>,
    summaries: &mut SummaryTable,
    summarize: bool,
) {
    let drained: Vec<OpenEntry> = open.drain().map(|(_, e)| e).collect();
    for entry in drained {
        let cls = classifications.remove(&entry.sig);
        emit_collapsed(entry, cls, control, out_tx, summaries, summarize).await;
    }
    classifications.clear();
    if summarize {
        flush_summaries(summaries, control, out_tx).await;
    }
    control.set_unique_signatures(0);
}

async fn emit_collapsed(
    entry: OpenEntry,
    classification: Option<Classification>,
    control: &Control,
    out_tx: &mpsc::Sender<ProcessedEvent>,
    summaries: &mut SummaryTable,
    summarize: bool,
) {
    // Singletons are already covered by the FirstOccurrence event we
    // emitted on arrival — dropping silently keeps the metric stream
    // honest about what "count" actually means.
    if entry.count <= 1 {
        return;
    }

    if summarize {
        if let Some(c) = &classification {
            if c.label == "routine" {
                summaries.observe(
                    &entry.source,
                    &entry.sig.to_string(),
                    entry.count,
                    &entry.sample,
                    entry.first_seen_unix,
                    entry.last_seen_unix,
                );
                return;
            }
        }
    }

    let window_secs = (entry.last_seen_unix - entry.first_seen_unix).max(0.0);
    let ev = ProcessedEvent::Collapsed {
        signature: entry.sig.to_string(),
        count: entry.count,
        window_secs,
        first_seen: entry.first_seen_unix,
        last_seen: entry.last_seen_unix,
        sample: entry.sample,
        source: entry.source,
        service: entry.service,
        classification,
    };
    if out_tx.send(ev).await.is_ok() {
        control.observe_out(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params(window_secs: u64) -> DedupParams {
        DedupParams {
            window_secs,
            max_open_signatures: 16,
            sidecar: None,
            summarize_routine: false,
            summary_flush_secs: 60,
            source_to_service: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn collapses_repeated_lines() {
        let (in_tx, in_rx) = mpsc::channel::<IngestLine>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);
        let control = Control::new();

        let params = default_params(1);
        let c2 = control.clone();
        let task = tokio::spawn(async move { run(params, c2, in_rx, out_tx).await });

        for _ in 0..10 {
            in_tx
                .send(IngestLine {
                    source: "test".into(),
                    text: "ERROR payment-api: boom".into(),
                    ts_unix: 1_700_000_000.0,
                })
                .await
                .unwrap();
        }

        let first = tokio::time::timeout(Duration::from_secs(1), out_rx.recv())
            .await
            .unwrap()
            .unwrap();
        match first {
            ProcessedEvent::FirstOccurrence { service, .. } => {
                assert_eq!(service, "payment-api");
            }
            other => panic!("expected FirstOccurrence, got {other:?}"),
        }

        tokio::time::sleep(Duration::from_millis(1500)).await;
        drop(in_tx);

        let collapsed = tokio::time::timeout(Duration::from_secs(2), out_rx.recv())
            .await
            .unwrap()
            .unwrap();
        match collapsed {
            ProcessedEvent::Collapsed {
                count, classification, service, ..
            } => {
                assert_eq!(count, 10);
                assert_eq!(service, "payment-api");
                assert!(classification.is_none(), "no sidecar configured");
            }
            other => panic!("expected Collapsed, got {other:?}"),
        }

        task.await.unwrap().unwrap();
        assert_eq!(control.snapshot().events_in, 10);
    }

    #[tokio::test]
    async fn distinct_signatures_each_get_first_occurrence() {
        let (in_tx, in_rx) = mpsc::channel::<IngestLine>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);
        let control = Control::new();

        let params = default_params(5);
        let task = tokio::spawn(async move { run(params, control, in_rx, out_tx).await });

        in_tx
            .send(IngestLine {
                source: "t".into(),
                text: "ERROR a".into(),
                ts_unix: 1.0,
            })
            .await
            .unwrap();
        in_tx
            .send(IngestLine {
                source: "t".into(),
                text: "INFO b".into(),
                ts_unix: 2.0,
            })
            .await
            .unwrap();

        let a = out_rx.recv().await.unwrap();
        let b = out_rx.recv().await.unwrap();
        assert!(matches!(a, ProcessedEvent::FirstOccurrence { .. }));
        assert!(matches!(b, ProcessedEvent::FirstOccurrence { .. }));

        drop(in_tx);
        task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn low_count_bursts_still_collapse() {
        // Three duplicate lines should still yield a Collapsed event with count=3.
        // (Regression guard for the "what if it's only 3, not 10000?" worry.)
        let (in_tx, in_rx) = mpsc::channel::<IngestLine>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);
        let control = Control::new();
        let task = tokio::spawn(async move {
            run(default_params(1), control, in_rx, out_tx).await
        });

        for _ in 0..3 {
            in_tx
                .send(IngestLine {
                    source: "t".into(),
                    text: "ERROR svc: rare blip".into(),
                    ts_unix: 1.0,
                })
                .await
                .unwrap();
        }

        // First event is the FirstOccurrence; drain it.
        let _first = out_rx.recv().await.unwrap();
        tokio::time::sleep(Duration::from_millis(1500)).await;
        drop(in_tx);

        let collapsed = tokio::time::timeout(Duration::from_secs(2), out_rx.recv())
            .await
            .unwrap()
            .unwrap();
        match collapsed {
            ProcessedEvent::Collapsed { count, .. } => assert_eq!(count, 3),
            other => panic!("expected Collapsed, got {other:?}"),
        }
        task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn singleton_emits_first_but_no_collapsed() {
        // count == 1 should NOT emit a Collapsed event (FirstOccurrence covers it).
        let (in_tx, in_rx) = mpsc::channel::<IngestLine>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);
        let control = Control::new();
        let task = tokio::spawn(async move {
            run(default_params(1), control, in_rx, out_tx).await
        });

        in_tx
            .send(IngestLine {
                source: "t".into(),
                text: "ERROR svc: one-off".into(),
                ts_unix: 1.0,
            })
            .await
            .unwrap();

        let first = out_rx.recv().await.unwrap();
        assert!(matches!(first, ProcessedEvent::FirstOccurrence { .. }));

        tokio::time::sleep(Duration::from_millis(1500)).await;
        drop(in_tx);

        let next = tokio::time::timeout(Duration::from_millis(500), out_rx.recv()).await;
        assert!(next.is_err() || next.unwrap().is_none());
        task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn source_to_service_hint_overrides_extraction() {
        let (in_tx, in_rx) = mpsc::channel::<IngestLine>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);
        let control = Control::new();
        let mut hints = HashMap::new();
        hints.insert("tcp://1.2.3.4:5140".to_string(), "us-east-payment".to_string());
        let params = DedupParams {
            window_secs: 1,
            max_open_signatures: 16,
            sidecar: None,
            summarize_routine: false,
            summary_flush_secs: 60,
            source_to_service: hints,
        };
        let task = tokio::spawn(async move { run(params, control, in_rx, out_tx).await });

        in_tx
            .send(IngestLine {
                source: "tcp://1.2.3.4:5140".into(),
                text: "ERROR other-name: ignored".into(),
                ts_unix: 1.0,
            })
            .await
            .unwrap();

        let first = out_rx.recv().await.unwrap();
        match first {
            ProcessedEvent::FirstOccurrence { service, .. } => {
                assert_eq!(service, "us-east-payment");
            }
            other => panic!("expected FirstOccurrence, got {other:?}"),
        }
        drop(in_tx);
        task.await.unwrap().unwrap();
    }
}
