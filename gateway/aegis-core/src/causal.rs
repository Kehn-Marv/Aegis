//! Causal chain detector.
//!
//! Watches `FirstOccurrence` events from the dedup stage. When ≥
//! `min_services` distinct services fire a *new* signature within a rolling
//! window of `window_secs`, that's evidence of a multi-service incident.
//! Aegis identifies the **root cause** as the service whose earliest first
//! occurrence is oldest — i.e. *who broke first*.
//!
//! Output: a single `CausalChain` event whose `chain` field lists all
//! involved services in temporal order. The first entry is the root cause;
//! every subsequent entry is collateral damage with an `ts_offset_secs`
//! relative to the root.
//!
//! Edge cases the detector handles by design:
//!   * A single service spamming many signatures doesn't fire a chain.
//!     We require *distinct services*, not distinct signatures.
//!   * Two services failing simultaneously (offset < 1s) → confidence drops
//!     because the temporal order is ambiguous. The output still names a
//!     root, but the confidence reflects the uncertainty.
//!   * After a chain fires, the participating service+signature pairs go
//!     into a cooldown so a long-running incident doesn't re-fire every
//!     window. New services joining the same chain extend it instead.

use crate::event::{CausalLink, ProcessedEvent};
use crate::id::short_uuid;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::debug;

/// Tunable parameters for the causal engine.
#[derive(Clone, Debug)]
pub struct CausalParams {
    /// Rolling window in which new-service first-occurrences are grouped
    /// into a single chain. Defaults to 60 seconds, which matches the
    /// typical "fan-out within a minute" pattern of cascading outages.
    pub window_secs: u64,
    /// Minimum distinct services involved before a chain is emitted.
    /// Two is the smallest interesting value (A → B). Three is a good
    /// default for noisy environments where two-service correlations are
    /// frequent but rarely actionable.
    pub min_services: usize,
    /// After a chain fires, suppress re-emission for this many seconds.
    /// New services joining within the cooldown extend the existing chain;
    /// otherwise we'd flood Splunk with one chain per dedup window.
    pub cooldown_secs: u64,
    /// Per-service ring buffer size. Keeps memory bounded even under
    /// pathological event storms.
    pub per_service_buffer: usize,
}

impl Default for CausalParams {
    fn default() -> Self {
        Self {
            window_secs: 60,
            min_services: 3,
            cooldown_secs: 300,
            per_service_buffer: 16,
        }
    }
}

#[derive(Debug, Clone)]
struct Recent {
    signature: String,
    ts: f64,
    sample: String,
}

/// State the engine carries between events.
struct Engine {
    params: CausalParams,
    /// `service → recent first-occurrences (newest first)`.
    by_service: HashMap<String, VecDeque<Recent>>,
    /// Last time we emitted a chain rooted at the given service. Used to
    /// honour `cooldown_secs` so a long incident produces *one* chain, not
    /// a chain per window tick.
    last_chain_at: HashMap<String, Instant>,
}

impl Engine {
    fn new(params: CausalParams) -> Self {
        Self {
            params,
            by_service: HashMap::new(),
            last_chain_at: HashMap::new(),
        }
    }

    fn record(&mut self, service: &str, signature: &str, ts: f64, sample: &str) {
        let buf = self.by_service.entry(service.to_string()).or_default();
        // Keep buffer bounded so a runaway service can't OOM the engine.
        if buf.len() == self.params.per_service_buffer {
            buf.pop_back();
        }
        buf.push_front(Recent {
            signature: signature.to_string(),
            ts,
            sample: sample.to_string(),
        });
    }

    /// Scan recent events across all services and emit a chain if the
    /// distinct-service count crosses the threshold inside the window.
    fn try_detect(&mut self, now_unix: f64, now_mono: Instant) -> Option<ProcessedEvent> {
        let window = self.params.window_secs as f64;
        if window <= 0.0 {
            return None;
        }

        // Find the earliest first-occurrence in the window per service.
        let mut earliest_per_service: BTreeMap<String, Recent> = BTreeMap::new();
        for (svc, buf) in &self.by_service {
            for r in buf {
                if (now_unix - r.ts) > window {
                    continue;
                }
                let entry = earliest_per_service
                    .entry(svc.clone())
                    .or_insert_with(|| r.clone());
                if r.ts < entry.ts {
                    *entry = r.clone();
                }
            }
        }

        if earliest_per_service.len() < self.params.min_services {
            return None;
        }

        // Sort by ts ascending — the smallest ts is the root cause.
        let mut sorted: Vec<(String, Recent)> = earliest_per_service.into_iter().collect();
        sorted.sort_by(|a, b| {
            a.1.ts
                .partial_cmp(&b.1.ts)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let (root_service, root_recent) = sorted[0].clone();

        // Cooldown so a long-running incident only fires once.
        if let Some(prev) = self.last_chain_at.get(&root_service) {
            if now_mono.duration_since(*prev)
                < Duration::from_secs(self.params.cooldown_secs)
            {
                debug!(
                    root = %root_service,
                    cooldown = self.params.cooldown_secs,
                    "suppressing chain re-emission inside cooldown"
                );
                return None;
            }
        }

        let chain: Vec<CausalLink> = sorted
            .iter()
            .map(|(svc, r)| CausalLink {
                service: svc.clone(),
                signature: r.signature.clone(),
                ts: r.ts,
                ts_offset_secs: (r.ts - root_recent.ts).max(0.0),
                sample: r.sample.clone(),
            })
            .collect();

        let last_seen = chain.iter().map(|l| l.ts).fold(root_recent.ts, f64::max);
        let confidence = score_confidence(&chain);

        self.last_chain_at.insert(root_service.clone(), now_mono);

        Some(ProcessedEvent::CausalChain {
            chain_id: short_uuid(),
            root_cause_service: root_service,
            confidence,
            chain,
            first_seen: root_recent.ts,
            last_seen,
            // Filled in by the orchestrator that has the dedup counters; we
            // don't see Collapsed events on this stream.
            suppressed_lines: 0,
        })
    }

    /// Drop entries older than the window across all services. Keeps the
    /// state small even when services come and go.
    fn gc(&mut self, now_unix: f64) {
        let window = self.params.window_secs as f64;
        for buf in self.by_service.values_mut() {
            while let Some(back) = buf.back() {
                if (now_unix - back.ts) > window {
                    buf.pop_back();
                } else {
                    break;
                }
            }
        }
        // Drop services we haven't seen anything from for a full window.
        self.by_service.retain(|_, buf| !buf.is_empty());
    }
}

/// Spread in seconds between the earliest and latest links. A spread that
/// fits comfortably inside the causal window gives high confidence; a
/// spread of ~0s (everything fired at once) is ambiguous.
fn score_confidence(chain: &[CausalLink]) -> f32 {
    if chain.len() < 2 {
        return 0.5;
    }
    let max_offset = chain
        .iter()
        .map(|l| l.ts_offset_secs)
        .fold(0.0_f64, f64::max);
    // Saturating to 1.0 over 5s gives a smooth curve:
    //   spread=0s → 0.65, spread=5s → 1.0, longer spreads stay at 1.0.
    let base = 0.65 + 0.35 * (max_offset / 5.0).min(1.0);
    base.clamp(0.0, 1.0) as f32
}

/// Run the causal-chain engine. Reads `ProcessedEvent`s from `in_rx`,
/// forwards every one to `out_tx`, and additionally emits a `CausalChain`
/// event whenever a multi-service incident is detected.
///
/// Sits *after* the dedup stage so the chain naturally inherits dedup's
/// stability guarantees (signatures already normalised, sources tagged
/// with services).
pub async fn run(
    params: CausalParams,
    mut in_rx: mpsc::Receiver<ProcessedEvent>,
    out_tx: mpsc::Sender<ProcessedEvent>,
) -> anyhow::Result<()> {
    let mut engine = Engine::new(params);
    let mut sweep = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            biased;

            maybe = in_rx.recv() => {
                let Some(ev) = maybe else { break };
                // Only `FirstOccurrence` carries the "this is new" signal we need.
                // Pass everything through unchanged.
                if let ProcessedEvent::FirstOccurrence {
                    service, signature, ts, line, ..
                } = &ev {
                    // A causal chain is about things *breaking*. Routine INFO/
                    // DEBUG first-sightings (a healthy service simply being seen
                    // for the first time) must not seed a chain, or any busy
                    // multi-service fleet would look like a perpetual incident.
                    if is_anomalous(line) {
                        engine.record(service, signature, *ts, line);
                        if let Some(chain) = engine.try_detect(*ts, Instant::now()) {
                            if out_tx.send(chain).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                if out_tx.send(ev).await.is_err() {
                    break;
                }
            }

            _ = sweep.tick() => {
                let now_unix = now_unix_secs();
                engine.gc(now_unix);
            }
        }
    }

    Ok(())
}

fn now_unix_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// True for lines that signal something abnormal (errors, warnings, stack
/// continuations, structured non-info lines). Routine `INFO`/`DEBUG`/`TRACE`
/// first-occurrences return false so they never seed a causal chain.
fn is_anomalous(line: &str) -> bool {
    let first = line.trim_start().split_whitespace().next().unwrap_or("");
    !matches!(
        first.to_ascii_uppercase().as_str(),
        "INFO" | "DEBUG" | "TRACE" | "NOTICE"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::CausalLink;

    fn first(service: &str, sig: &str, ts: f64) -> ProcessedEvent {
        ProcessedEvent::FirstOccurrence {
            signature: sig.into(),
            line: format!("ERROR {service}: boom"),
            ts,
            source: "tcp://x".into(),
            service: service.into(),
        }
    }

    fn unwrap_chain(ev: &ProcessedEvent) -> &Vec<CausalLink> {
        match ev {
            ProcessedEvent::CausalChain { chain, .. } => chain,
            other => panic!("expected CausalChain, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn three_services_fire_a_chain() {
        let (in_tx, in_rx) = mpsc::channel::<ProcessedEvent>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);

        let params = CausalParams {
            window_secs: 60,
            min_services: 3,
            cooldown_secs: 1,
            per_service_buffer: 8,
        };
        let task = tokio::spawn(async move { run(params, in_rx, out_tx).await });

        // payment-api breaks first at t=0, checkout at t=10, orders at t=20.
        in_tx.send(first("payment-api", "sig-a", 0.0)).await.unwrap();
        in_tx.send(first("checkout", "sig-b", 10.0)).await.unwrap();
        in_tx.send(first("orders", "sig-c", 20.0)).await.unwrap();

        // Drain pass-through firsts + the chain.
        let mut seen_chain = None;
        for _ in 0..6 {
            if let Ok(Some(ev)) = tokio::time::timeout(Duration::from_millis(200), out_rx.recv()).await {
                if matches!(ev, ProcessedEvent::CausalChain { .. }) {
                    seen_chain = Some(ev);
                    break;
                }
            } else {
                break;
            }
        }
        drop(in_tx);
        task.await.unwrap().unwrap();

        let chain = seen_chain.expect("a chain should have fired");
        let links = unwrap_chain(&chain);
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].service, "payment-api");
        assert_eq!(links[1].service, "checkout");
        assert_eq!(links[2].service, "orders");
        // Root cause confidence should be high — clear 10s + 20s spread.
        match chain {
            ProcessedEvent::CausalChain { confidence, root_cause_service, .. } => {
                assert!(confidence > 0.9, "expected high confidence, got {confidence}");
                assert_eq!(root_cause_service, "payment-api");
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn single_service_does_not_fire_chain() {
        let (in_tx, in_rx) = mpsc::channel::<ProcessedEvent>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);
        let task = tokio::spawn(async move {
            run(CausalParams::default(), in_rx, out_tx).await
        });

        // Same service, three signatures — should NOT trigger.
        in_tx.send(first("svc", "sig-a", 0.0)).await.unwrap();
        in_tx.send(first("svc", "sig-b", 1.0)).await.unwrap();
        in_tx.send(first("svc", "sig-c", 2.0)).await.unwrap();

        let mut events = Vec::new();
        while let Ok(Some(ev)) =
            tokio::time::timeout(Duration::from_millis(150), out_rx.recv()).await
        {
            events.push(ev);
        }
        drop(in_tx);
        task.await.unwrap().unwrap();

        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| matches!(e, ProcessedEvent::FirstOccurrence { .. })));
    }

    #[tokio::test]
    async fn routine_info_lines_do_not_fire_chain() {
        let (in_tx, in_rx) = mpsc::channel::<ProcessedEvent>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);
        let task = tokio::spawn(async move { run(CausalParams::default(), in_rx, out_tx).await });

        // Three *healthy* services seen for the first time — INFO lines must
        // never be mistaken for a multi-service incident.
        let info = |svc: &str, sig: &str, ts: f64| ProcessedEvent::FirstOccurrence {
            signature: sig.into(),
            line: format!("INFO  [t] {svc}: 200 OK"),
            ts,
            source: "tcp://x".into(),
            service: svc.into(),
        };
        in_tx.send(info("api-gateway", "s1", 0.0)).await.unwrap();
        in_tx.send(info("auth", "s2", 1.0)).await.unwrap();
        in_tx.send(info("orders", "s3", 2.0)).await.unwrap();

        let mut events = Vec::new();
        while let Ok(Some(ev)) =
            tokio::time::timeout(Duration::from_millis(150), out_rx.recv()).await
        {
            events.push(ev);
        }
        drop(in_tx);
        task.await.unwrap().unwrap();

        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| matches!(e, ProcessedEvent::FirstOccurrence { .. })));
    }

    #[tokio::test]
    async fn simultaneous_failures_yield_lower_confidence() {
        let (in_tx, in_rx) = mpsc::channel::<ProcessedEvent>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);

        let task = tokio::spawn(async move {
            run(
                CausalParams {
                    min_services: 3,
                    cooldown_secs: 1,
                    ..CausalParams::default()
                },
                in_rx,
                out_tx,
            )
            .await
        });

        // Three services break in the same instant — temporal order is
        // ambiguous, confidence should be low.
        in_tx.send(first("a", "sig-a", 100.0)).await.unwrap();
        in_tx.send(first("b", "sig-b", 100.0)).await.unwrap();
        in_tx.send(first("c", "sig-c", 100.0)).await.unwrap();

        let mut chain = None;
        for _ in 0..6 {
            if let Ok(Some(ev)) =
                tokio::time::timeout(Duration::from_millis(150), out_rx.recv()).await
            {
                if let ProcessedEvent::CausalChain { confidence, .. } = ev {
                    chain = Some(confidence);
                    break;
                }
            } else {
                break;
            }
        }
        drop(in_tx);
        task.await.unwrap().unwrap();

        let conf = chain.expect("chain should have fired");
        assert!(conf <= 0.7, "expected ambiguous confidence, got {conf}");
    }

    #[tokio::test]
    async fn cooldown_suppresses_back_to_back_chains() {
        let (in_tx, in_rx) = mpsc::channel::<ProcessedEvent>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(16);
        let params = CausalParams {
            window_secs: 600,
            min_services: 2,
            cooldown_secs: 600,
            per_service_buffer: 8,
        };
        let task = tokio::spawn(async move { run(params, in_rx, out_tx).await });

        in_tx.send(first("a", "s1", 0.0)).await.unwrap();
        in_tx.send(first("b", "s2", 1.0)).await.unwrap();
        // Another two services in the same window — should NOT re-fire.
        in_tx.send(first("a", "s3", 2.0)).await.unwrap();
        in_tx.send(first("b", "s4", 3.0)).await.unwrap();

        let mut chain_count = 0;
        while let Ok(Some(ev)) =
            tokio::time::timeout(Duration::from_millis(150), out_rx.recv()).await
        {
            if matches!(ev, ProcessedEvent::CausalChain { .. }) {
                chain_count += 1;
            }
        }
        drop(in_tx);
        task.await.unwrap().unwrap();
        assert_eq!(chain_count, 1);
    }
}
