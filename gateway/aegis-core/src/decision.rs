//! Decision engine — turns raw signals into the focused card an engineer reads.
//!
//! The decision card replaces the old "Execute" button. Instead of asking
//! the human to pick a tool, Aegis presents:
//!
//!   * the current health state (green / orange / red),
//!   * the probable root cause (with confidence),
//!   * the most similar past incident, including the fix that worked,
//!   * one concrete next step (where to look),
//!   * a one-line business-impact sentence for the affected service,
//!   * three buttons: `I'm on it`, `Show me more`, `This looks different`.
//!
//! The engine itself is intentionally a thin synthesiser. The hard work
//! (signature dedup, causal attribution, similarity ranking, silence
//! detection) happens upstream; this module just picks a shape and writes
//! the plain-English copy.

use crate::event::{CausalLink, HealthState, IncidentMatch, ProcessedEvent};
use crate::id::short_uuid;
use crate::incident_memory::Store;
use crate::service_catalog::ServiceCatalog;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionParams {
    /// How many similar past incidents to surface alongside the headline.
    pub max_similar_incidents: usize,
    /// Minimum similarity (0.0–1.0) before a past incident is shown.
    pub min_similarity: f32,
    /// Idle period before the engine downshifts the state back to Green.
    /// Defaults to 5 minutes — long enough that a momentary lull doesn't
    /// flap the badge, short enough that engineers see "ok now" quickly.
    pub idle_to_green_secs: u64,
}

impl Default for DecisionParams {
    fn default() -> Self {
        Self {
            max_similar_incidents: 3,
            min_similarity: 0.25,
            idle_to_green_secs: 300,
        }
    }
}

#[derive(Clone)]
pub struct DecisionEngine {
    params: DecisionParams,
    store: Store,
    catalog: ServiceCatalog,
}

impl DecisionEngine {
    pub fn new(params: DecisionParams, store: Store, catalog: ServiceCatalog) -> Self {
        Self {
            params,
            store,
            catalog,
        }
    }

    /// Synthesize a red-state card from a freshly fired causal chain.
    /// Also persists the chain as a fingerprint in the memory store.
    pub fn on_chain(&self, chain: &ProcessedEvent) -> anyhow::Result<ProcessedEvent> {
        let ProcessedEvent::CausalChain {
            chain_id,
            root_cause_service,
            confidence,
            chain: links,
            last_seen,
            ..
        } = chain
        else {
            anyhow::bail!("on_chain requires a CausalChain event");
        };

        let similar = match self
            .store
            .search_similar(chain, self.params.max_similar_incidents, self.params.min_similarity)
        {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "incident memory search failed; continuing without past matches");
                Vec::new()
            }
        };

        let headline = headline_for_chain(root_cause_service, links, *confidence);
        let suggested_next_step = suggest_next_step(root_cause_service, links, &similar);
        let business_impact = self.catalog.lookup(root_cause_service).map(str::to_string);

        Ok(ProcessedEvent::DecisionCard {
            decision_id: short_uuid(),
            ts: *last_seen,
            state: HealthState::Red,
            chain_id: Some(chain_id.clone()),
            root_cause_service: Some(root_cause_service.clone()),
            headline,
            suggested_next_step,
            business_impact,
            similar_incidents: similar,
        })
    }

    /// Synthesize an orange-state card from a single service that's
    /// misbehaving but hasn't (yet) dragged others down with it.
    pub fn on_orange(&self, service: &str, sample: &str) -> ProcessedEvent {
        let headline = format!(
            "{service} is misbehaving. No multi-service chain detected yet, but volume is rising."
        );
        let suggested_next_step = format!(
            "Check {service} health and recent deploys. Aegis will escalate to red if other services start failing."
        );
        let business_impact = self.catalog.lookup(service).map(str::to_string);
        ProcessedEvent::DecisionCard {
            decision_id: short_uuid(),
            ts: now_unix_secs(),
            state: HealthState::Orange,
            chain_id: None,
            root_cause_service: Some(service.to_string()),
            headline,
            suggested_next_step: format!("{suggested_next_step}\nSample: {}", trim_sample(sample)),
            business_impact,
            similar_incidents: Vec::new(),
        }
    }

    /// Build a green card when state has been clean for a while.
    pub fn green_card(&self) -> ProcessedEvent {
        ProcessedEvent::DecisionCard {
            decision_id: short_uuid(),
            ts: now_unix_secs(),
            state: HealthState::Green,
            chain_id: None,
            root_cause_service: None,
            headline: "All quiet. Dedup is working and no causal chains are active.".into(),
            suggested_next_step: "Nothing to do. Aegis is watching for first-fire patterns.".into(),
            business_impact: None,
            similar_incidents: Vec::new(),
        }
    }
}

/// Pipeline stage: every chain entering the engine produces a card.
///
/// The stage forwards every input event downstream untouched **plus**, for
/// each `CausalChain` it observes, emits an `IncidentMemory` snapshot
/// (open, no resolution yet) immediately followed by a `DecisionCard`.
pub async fn run(
    params: DecisionParams,
    store: Store,
    catalog: ServiceCatalog,
    mut in_rx: mpsc::Receiver<ProcessedEvent>,
    out_tx: mpsc::Sender<ProcessedEvent>,
    card_tx: tokio::sync::watch::Sender<Option<ProcessedEvent>>,
) -> anyhow::Result<()> {
    let engine = DecisionEngine::new(params.clone(), store.clone(), catalog);
    let mut last_chain_at: Option<std::time::Instant> = None;
    let mut idle_sweep = interval(Duration::from_secs(30));
    idle_sweep.tick().await; // skip immediate fire

    while let Some(ev) = tokio::select! {
        maybe = in_rx.recv() => maybe.map(InOrTick::Event),
        _ = idle_sweep.tick() => Some(InOrTick::Sweep),
    } {
        match ev {
            InOrTick::Sweep => {
                if let Some(last) = last_chain_at {
                    if last.elapsed().as_secs() >= params.idle_to_green_secs {
                        let card = engine.green_card();
                        let _ = card_tx.send(Some(card.clone()));
                        if out_tx.send(card).await.is_err() {
                            break;
                        }
                        last_chain_at = None;
                    }
                }
            }
            InOrTick::Event(ev) => {
                if let ProcessedEvent::CausalChain { .. } = &ev {
                    match store.record_chain(&ev) {
                        Ok(id) => debug!(incident_id = %id, "recorded new chain in memory"),
                        Err(e) => warn!(error = %e, "failed to record chain in memory"),
                    }
                    if let Some(fp) = store.recent(1).ok().and_then(|v| v.into_iter().next()) {
                        if out_tx.send(fp.to_event()).await.is_err() {
                            break;
                        }
                    }
                    match engine.on_chain(&ev) {
                        Ok(card) => {
                            let _ = card_tx.send(Some(card.clone()));
                            if out_tx.send(card).await.is_err() {
                                break;
                            }
                            last_chain_at = Some(std::time::Instant::now());
                        }
                        Err(e) => warn!(error = %e, "decision synthesis failed"),
                    }
                }
                if out_tx.send(ev).await.is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

enum InOrTick {
    Event(ProcessedEvent),
    Sweep,
}

fn headline_for_chain(root: &str, links: &[CausalLink], confidence: f32) -> String {
    if links.len() < 2 {
        return format!("{root} fired a new pattern.");
    }
    let first_ts = links[0].ts;
    let mut sentences: Vec<String> = Vec::new();
    sentences.push(format!("{} broke first.", root));
    for link in links.iter().skip(1) {
        let delta = (link.ts - first_ts).max(0.0);
        sentences.push(format!(
            "{} followed {} later.",
            link.service,
            format_offset(delta)
        ));
    }
    let confidence_pct = (confidence * 100.0).round() as i32;
    sentences.push(format!("Root cause: {root} ({confidence_pct}% confidence)."));
    sentences.join(" ")
}

fn suggest_next_step(
    root: &str,
    links: &[CausalLink],
    similar: &[IncidentMatch],
) -> String {
    // Prefer the highest-similarity match that already has a resolution
    // card filled in — that's the *useful* one. Fall back to the top
    // match overall, then to the generic "check service health" line.
    let best_resolved = similar
        .iter()
        .find(|m| m.past_cause.is_some() && m.past_fix.is_some());
    if let Some(best) = best_resolved {
        if let (Some(cause), Some(fix)) = (&best.past_cause, &best.past_fix) {
            let similarity_pct = (best.similarity * 100.0).round() as i32;
            let resolved_in = best
                .past_resolved_in_minutes
                .map(|m| format!(" (fixed in {m} min last time)"))
                .unwrap_or_default();
            return format!(
                "This looks {similarity_pct}% similar to a past incident{resolved_in}. \
                 Last time the cause was: \"{cause}\". The fix was: \"{fix}\". \
                 Start by verifying that."
            );
        }
    }
    if let Some(best) = similar.first() {
        let similarity_pct = (best.similarity * 100.0).round() as i32;
        return format!(
            "This looks {similarity_pct}% similar to a past incident — but no one ever \
             filled in what fixed it. Check {root} health and recent deploys; please \
             record a resolution card when you fix this one so the next on-call has \
             a head start."
        );
    }
    let sample = links
        .first()
        .map(|l| trim_sample(&l.sample))
        .unwrap_or_default();
    if sample.is_empty() {
        format!("Check {root} health, recent deploys, and dependency status. This is the first time Aegis has seen this incident shape, so please record a resolution card when you fix it.")
    } else {
        format!(
            "Check {root} health and recent deploys. Earliest signal: {sample}. This is the first time Aegis has seen this incident shape; please record a resolution card when you fix it."
        )
    }
}

fn format_offset(delta_secs: f64) -> String {
    if delta_secs < 1.0 {
        "almost immediately".to_string()
    } else if delta_secs < 60.0 {
        format!("{:.0}s", delta_secs)
    } else {
        format!("{:.1} min", delta_secs / 60.0)
    }
}

fn trim_sample(sample: &str) -> String {
    let mut trimmed = sample.trim().replace(['\n', '\r'], " ");
    if trimmed.len() > 140 {
        trimmed.truncate(137);
        trimmed.push_str("...");
    }
    trimmed
}

fn now_unix_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::CausalLink;

    fn chain_ev() -> ProcessedEvent {
        ProcessedEvent::CausalChain {
            chain_id: "ch1".into(),
            root_cause_service: "payment-api".into(),
            confidence: 0.9,
            chain: vec![
                CausalLink {
                    service: "payment-api".into(),
                    signature: "s1".into(),
                    ts: 0.0,
                    ts_offset_secs: 0.0,
                    sample: "ERROR payment-api: db pool exhausted".into(),
                },
                CausalLink {
                    service: "checkout".into(),
                    signature: "s2".into(),
                    ts: 26.0,
                    ts_offset_secs: 26.0,
                    sample: "ERROR checkout: payment-api timeout".into(),
                },
            ],
            first_seen: 0.0,
            last_seen: 26.0,
            suppressed_lines: 1000,
        }
    }

    #[test]
    fn headline_lists_root_then_followers() {
        let chain = chain_ev();
        let links = match &chain {
            ProcessedEvent::CausalChain { chain, .. } => chain.clone(),
            _ => unreachable!(),
        };
        let h = headline_for_chain("payment-api", &links, 0.89);
        assert!(h.contains("payment-api broke first"));
        assert!(h.contains("checkout followed"));
        assert!(h.contains("89% confidence"));
    }

    #[test]
    fn engine_emits_red_card_with_business_impact() {
        let store = Store::open(":memory:").unwrap();
        let catalog = ServiceCatalog::with_entries(&[
            ("payment-api", "Handles all transaction processing."),
        ]);
        let engine = DecisionEngine::new(DecisionParams::default(), store, catalog);
        let card = engine.on_chain(&chain_ev()).unwrap();
        match card {
            ProcessedEvent::DecisionCard {
                state,
                root_cause_service,
                business_impact,
                ..
            } => {
                assert_eq!(state, HealthState::Red);
                assert_eq!(root_cause_service.as_deref(), Some("payment-api"));
                assert!(business_impact.unwrap().contains("transaction"));
            }
            _ => panic!("expected DecisionCard"),
        }
    }

    #[test]
    fn engine_surfaces_past_fix_when_present() {
        let store = Store::open(":memory:").unwrap();

        let past = chain_ev();
        let id = store.record_chain(&past).unwrap();
        store
            .resolve(
                &id,
                crate::incident_memory::ResolutionCard {
                    cause: "Connection pool exhausted".into(),
                    fix: "Reduced retry interval to 30s".into(),
                },
                60.0,
            )
            .unwrap();

        let new = {
            let mut clone = chain_ev();
            if let ProcessedEvent::CausalChain { chain_id, .. } = &mut clone {
                *chain_id = "different".into();
            }
            clone
        };

        let engine = DecisionEngine::new(DecisionParams::default(), store, ServiceCatalog::default());
        let card = engine.on_chain(&new).unwrap();
        match card {
            ProcessedEvent::DecisionCard {
                similar_incidents,
                suggested_next_step,
                ..
            } => {
                assert!(!similar_incidents.is_empty());
                assert!(suggested_next_step.contains("Connection pool exhausted"));
                assert!(suggested_next_step.contains("retry interval"));
            }
            _ => panic!("expected DecisionCard"),
        }
    }

    #[test]
    fn engine_handles_first_time_incident_gracefully() {
        // No past incidents in memory → the card should still render, with
        // a sensible "first time we've seen this" next-step nudge.
        let store = Store::open(":memory:").unwrap();
        let engine = DecisionEngine::new(DecisionParams::default(), store, ServiceCatalog::default());
        let card = engine.on_chain(&chain_ev()).unwrap();
        match card {
            ProcessedEvent::DecisionCard {
                similar_incidents,
                suggested_next_step,
                ..
            } => {
                assert!(similar_incidents.is_empty());
                assert!(suggested_next_step.to_lowercase().contains("first time"));
                assert!(suggested_next_step.contains("resolution card"));
            }
            _ => panic!("expected DecisionCard"),
        }
    }

    #[test]
    fn engine_prefers_resolved_match_over_unresolved() {
        let store = Store::open(":memory:").unwrap();

        // Two past chains; only the first is resolved. The engine must
        // prefer the resolved one when phrasing the next-step suggestion.
        let resolved = chain_ev();
        let resolved_id = store.record_chain(&resolved).unwrap();
        store
            .resolve(
                &resolved_id,
                crate::incident_memory::ResolutionCard {
                    cause: "Cache stampede".into(),
                    fix: "Added jittered exponential backoff to retries".into(),
                },
                30.0,
            )
            .unwrap();

        let unresolved = {
            let mut clone = chain_ev();
            if let ProcessedEvent::CausalChain { chain_id, .. } = &mut clone {
                *chain_id = "unresolved".into();
            }
            clone
        };
        store.record_chain(&unresolved).unwrap();

        let new = {
            let mut clone = chain_ev();
            if let ProcessedEvent::CausalChain { chain_id, .. } = &mut clone {
                *chain_id = "now".into();
            }
            clone
        };

        let engine = DecisionEngine::new(DecisionParams::default(), store, ServiceCatalog::default());
        let card = engine.on_chain(&new).unwrap();
        match card {
            ProcessedEvent::DecisionCard {
                suggested_next_step, ..
            } => {
                assert!(suggested_next_step.contains("Cache stampede"));
                assert!(suggested_next_step.contains("jittered"));
            }
            _ => panic!("expected DecisionCard"),
        }
    }

    #[test]
    fn green_card_is_neutral() {
        let engine = DecisionEngine::new(
            DecisionParams::default(),
            Store::open(":memory:").unwrap(),
            ServiceCatalog::default(),
        );
        let card = engine.green_card();
        match card {
            ProcessedEvent::DecisionCard { state, .. } => assert_eq!(state, HealthState::Green),
            _ => panic!("expected DecisionCard"),
        }
    }
}
