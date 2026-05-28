//! Events that flow through the Aegis pipeline.
//!
//! Aegis emits five sourcetypes into Splunk:
//!
//! | sourcetype          | when                                                  |
//! |---------------------|-------------------------------------------------------|
//! | `aegis:raw`         | first occurrence of a signature, or override passthrough |
//! | `aegis:metric`      | dedup window closed for a repeating signature         |
//! | `aegis:summary`     | routine-classified traffic rolled up per source        |
//! | `aegis:causal`      | multi-service incident with a probable root cause      |
//! | `aegis:decision`    | the focused "next step" card the engineer sees         |
//! | `aegis:incident`    | a memory entry: fingerprint + resolution (if filled in)|
//! | `aegis:silent`      | a service that was talking has gone quiet              |
//!
//! Self-metrics still flow under `aegis:selfmetric` from `self_metrics.rs`.

use crate::hec::HecEvent;
use crate::signature::Signature;
use serde::{Deserialize, Serialize};

/// Priority used by the SQLite queue. Lower number = drains first.
///
/// `HIGH` carries anomaly evidence (first-occurrence raws and override
/// passthrough) and the decision/causal/silent events that operators
/// need *during* an incident. `MEDIUM` is dedup metrics. `LOW` is bulk
/// summary traffic that's fine to lag during congestion.
pub const PRIORITY_HIGH: i64 = 0;
pub const PRIORITY_MEDIUM: i64 = 1;
pub const PRIORITY_LOW: i64 = 2;

/// Output of the AI sidecar's classifier, attached to events when available.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Classification {
    pub label: String,
    pub confidence: f32,
    pub strategy: String,
}

/// A raw line as received by the ingest layer.
#[derive(Clone, Debug)]
pub struct IngestLine {
    /// Free-form identifier for the source (e.g. `"tcp://127.0.0.1:54231"`).
    pub source: String,
    /// The log line, with the trailing newline already stripped.
    pub text: String,
    /// Wall-clock arrival time, unix epoch seconds (sub-second precision).
    pub ts_unix: f64,
}

/// One entry in a causal chain: a service that broke at some offset from
/// the chain's root cause.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CausalLink {
    pub service: String,
    pub signature: String,
    /// Wall-clock time the service first fired in this window.
    pub ts: f64,
    /// Seconds after the root cause this link appeared. `0.0` for the root.
    pub ts_offset_secs: f64,
    pub sample: String,
}

/// A match against a past incident in the memory store.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IncidentMatch {
    pub incident_id: String,
    /// `0.0` to `1.0` — how similar the new incident is to the historical one.
    pub similarity: f32,
    /// Wall-clock unix seconds of the historical incident.
    pub past_ts: f64,
    pub past_root_cause_service: String,
    /// Cause text the engineer filled in when resolving the past incident.
    /// `None` if the past incident is still open (we matched on shape alone).
    pub past_cause: Option<String>,
    /// Fix text from the past incident's resolution card.
    pub past_fix: Option<String>,
    /// Minutes it took to resolve the historical incident.
    pub past_resolved_in_minutes: Option<i64>,
}

/// Overall health colour the dashboard / UI surfaces above the fold.
///
/// Mirrors what Splunk customers asked for in the official observability
/// buyer's guide: *"Green is fine, red is too late. I want to know when
/// it's orange."*
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    /// Everything quiet, dedup working, no chains active.
    Green,
    /// Trending bad — anomaly rate climbing, queue growing, forecast crossing,
    /// or a single service is misbehaving. No multi-service chain yet.
    Orange,
    /// Active incident — a multi-service chain has fired or a service has gone
    /// silent inside the alert horizon.
    Red,
}

impl HealthState {
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthState::Green => "green",
            HealthState::Orange => "orange",
            HealthState::Red => "red",
        }
    }
}

/// What the dedup + causal + memory engines emit downstream toward the sink.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProcessedEvent {
    /// First time this signature has been seen in the current window.
    /// Forwarded raw so the operator has full incident context.
    FirstOccurrence {
        signature: String,
        line: String,
        ts: f64,
        source: String,
        /// Extracted service name (or the source string when extraction failed).
        service: String,
    },

    /// Window closed for a previously-seen signature with `count > 1`.
    /// Collapsed into a single lightweight metric event.
    Collapsed {
        signature: String,
        count: u64,
        window_secs: f64,
        first_seen: f64,
        last_seen: f64,
        sample: String,
        source: String,
        service: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        classification: Option<Classification>,
    },

    /// Raw passthrough event (override mode is active).
    Raw {
        line: String,
        ts: f64,
        source: String,
        service: String,
    },

    /// Periodic aggregate of routine-classified traffic for a single source.
    Summary {
        source: String,
        window_secs: f64,
        first_seen: f64,
        last_seen: f64,
        suppressed_lines: u64,
        unique_signatures: u64,
        top_signatures: Vec<TopSig>,
    },

    /// Several services started failing inside the causal window. The
    /// service whose anomaly appeared first is the probable root cause.
    CausalChain {
        /// Stable identifier for this chain. Re-used as the incident memory
        /// fingerprint id when (and only when) an engineer resolves it.
        chain_id: String,
        root_cause_service: String,
        /// Aegis's own confidence in the root-cause attribution (0.0–1.0).
        confidence: f32,
        /// All services involved in order of first appearance. The first
        /// element is the root cause; the rest are collateral damage.
        chain: Vec<CausalLink>,
        first_seen: f64,
        last_seen: f64,
        /// Total raw lines that fed this chain across all involved services.
        suppressed_lines: u64,
    },

    /// The focused recommendation a human acts on. Replaces the old
    /// "Execute" button on the UI — engineers click `I'm on it`,
    /// `Show me more`, or `This looks different`.
    DecisionCard {
        decision_id: String,
        ts: f64,
        state: HealthState,
        /// `None` only when state is `Green` (nothing to act on).
        chain_id: Option<String>,
        root_cause_service: Option<String>,
        /// Aegis's plain-English summary of what's happening.
        headline: String,
        /// One-line guidance: where to look first.
        suggested_next_step: String,
        /// Plain-English business context for the root-cause service, when
        /// known. Surfaces the "what does this mean for the business?"
        /// question Splunk's buyer's guide calls out.
        business_impact: Option<String>,
        /// Top similar past incidents. May be empty when this kind of
        /// incident has never been seen before.
        similar_incidents: Vec<IncidentMatch>,
    },

    /// A new fingerprint was written to the incident-memory store.
    /// Shipped to Splunk so operators can audit what Aegis is learning
    /// without poking the local SQLite file.
    IncidentMemory {
        incident_id: String,
        chain_id: String,
        ts: f64,
        root_cause_service: String,
        services: Vec<String>,
        /// Resolution card text (filled when the engineer submits it).
        cause: Option<String>,
        fix: Option<String>,
        resolved_at: Option<f64>,
        resolved_in_minutes: Option<i64>,
    },

    /// A service that was previously talking to us has gone quiet. Silence
    /// is itself a signal — most observability tools miss this because it
    /// is the *absence* of events, not the presence of one.
    ServiceSilent {
        service: String,
        last_seen: f64,
        silence_secs: f64,
        last_sample: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TopSig {
    pub signature: String,
    pub count: u64,
    pub sample: String,
}

impl ProcessedEvent {
    pub fn signature_hex(sig: &Signature) -> String {
        sig.to_string()
    }

    /// Sourcetype this event should land under in Splunk HEC.
    pub fn sourcetype(&self) -> &'static str {
        match self {
            ProcessedEvent::FirstOccurrence { .. } | ProcessedEvent::Raw { .. } => "aegis:raw",
            ProcessedEvent::Collapsed { .. } => "aegis:metric",
            ProcessedEvent::Summary { .. } => "aegis:summary",
            ProcessedEvent::CausalChain { .. } => "aegis:causal",
            ProcessedEvent::DecisionCard { .. } => "aegis:decision",
            ProcessedEvent::IncidentMemory { .. } => "aegis:incident",
            ProcessedEvent::ServiceSilent { .. } => "aegis:silent",
        }
    }

    /// Drain priority in the queue. Anomaly evidence and operator-facing
    /// signals drain first; metrics next; summaries last.
    pub fn priority(&self) -> i64 {
        match self {
            ProcessedEvent::FirstOccurrence { .. }
            | ProcessedEvent::Raw { .. }
            | ProcessedEvent::CausalChain { .. }
            | ProcessedEvent::DecisionCard { .. }
            | ProcessedEvent::ServiceSilent { .. } => PRIORITY_HIGH,
            ProcessedEvent::Collapsed { .. } | ProcessedEvent::IncidentMemory { .. } => {
                PRIORITY_MEDIUM
            }
            ProcessedEvent::Summary { .. } => PRIORITY_LOW,
        }
    }

    /// Wall-clock time on the event (unix epoch seconds, sub-second precision).
    pub fn event_time(&self) -> f64 {
        match self {
            ProcessedEvent::FirstOccurrence { ts, .. } | ProcessedEvent::Raw { ts, .. } => *ts,
            ProcessedEvent::Collapsed { last_seen, .. }
            | ProcessedEvent::Summary { last_seen, .. } => *last_seen,
            ProcessedEvent::CausalChain { last_seen, .. } => *last_seen,
            ProcessedEvent::DecisionCard { ts, .. }
            | ProcessedEvent::IncidentMemory { ts, .. } => *ts,
            ProcessedEvent::ServiceSilent { last_seen, silence_secs, .. } => last_seen + silence_secs,
        }
    }

    /// Source identifier passed through to HEC (`source` field). For
    /// service-scoped events we report the service name as the source so
    /// SPL filters like `source=payment-api` "just work" in dashboards.
    pub fn source(&self) -> &str {
        match self {
            ProcessedEvent::FirstOccurrence { source, .. }
            | ProcessedEvent::Raw { source, .. }
            | ProcessedEvent::Collapsed { source, .. }
            | ProcessedEvent::Summary { source, .. } => source,
            ProcessedEvent::CausalChain { root_cause_service, .. } => root_cause_service,
            ProcessedEvent::DecisionCard { root_cause_service, .. } => root_cause_service
                .as_deref()
                .unwrap_or("aegis"),
            ProcessedEvent::IncidentMemory { root_cause_service, .. } => root_cause_service,
            ProcessedEvent::ServiceSilent { service, .. } => service,
        }
    }

    /// Build an HEC payload for this event.
    pub fn to_hec_event(&self, host: &str, index: Option<&str>) -> HecEvent {
        let payload = match self {
            ProcessedEvent::FirstOccurrence {
                signature,
                line,
                service,
                ..
            } => serde_json::json!({
                "kind": "first_occurrence",
                "signature": signature,
                "line": line,
                "service": service,
            }),

            ProcessedEvent::Raw { line, service, .. } => serde_json::json!({
                "kind": "raw",
                "line": line,
                "service": service,
            }),

            ProcessedEvent::Collapsed {
                signature,
                count,
                window_secs,
                first_seen,
                last_seen,
                sample,
                classification,
                service,
                ..
            } => {
                let mut payload = serde_json::json!({
                    "kind": "collapsed",
                    "signature": signature,
                    "count": count,
                    "window_secs": window_secs,
                    "first_seen": first_seen,
                    "last_seen": last_seen,
                    "sample": sample,
                    "service": service,
                });
                if let Some(c) = classification {
                    payload["classification"] = serde_json::json!({
                        "label": c.label,
                        "confidence": c.confidence,
                        "strategy": c.strategy,
                    });
                }
                payload
            }

            ProcessedEvent::Summary {
                window_secs,
                first_seen,
                last_seen,
                suppressed_lines,
                unique_signatures,
                top_signatures,
                ..
            } => serde_json::json!({
                "kind": "summary",
                "window_secs": window_secs,
                "first_seen": first_seen,
                "last_seen": last_seen,
                "suppressed_lines": suppressed_lines,
                "unique_signatures": unique_signatures,
                "top_signatures": top_signatures,
            }),

            ProcessedEvent::CausalChain {
                chain_id,
                root_cause_service,
                confidence,
                chain,
                first_seen,
                last_seen,
                suppressed_lines,
            } => serde_json::json!({
                "kind": "causal_chain",
                "chain_id": chain_id,
                "root_cause_service": root_cause_service,
                "confidence": confidence,
                "chain": chain,
                "first_seen": first_seen,
                "last_seen": last_seen,
                "suppressed_lines": suppressed_lines,
                "services_involved": chain.len(),
            }),

            ProcessedEvent::DecisionCard {
                decision_id,
                ts,
                state,
                chain_id,
                root_cause_service,
                headline,
                suggested_next_step,
                business_impact,
                similar_incidents,
            } => serde_json::json!({
                "kind": "decision_card",
                "decision_id": decision_id,
                "ts": ts,
                "state": state.as_str(),
                "chain_id": chain_id,
                "root_cause_service": root_cause_service,
                "headline": headline,
                "suggested_next_step": suggested_next_step,
                "business_impact": business_impact,
                "similar_incidents": similar_incidents,
                "similar_count": similar_incidents.len(),
            }),

            ProcessedEvent::IncidentMemory {
                incident_id,
                chain_id,
                ts,
                root_cause_service,
                services,
                cause,
                fix,
                resolved_at,
                resolved_in_minutes,
            } => serde_json::json!({
                "kind": "incident_memory",
                "incident_id": incident_id,
                "chain_id": chain_id,
                "ts": ts,
                "root_cause_service": root_cause_service,
                "services": services,
                "cause": cause,
                "fix": fix,
                "resolved_at": resolved_at,
                "resolved_in_minutes": resolved_in_minutes,
                "resolved": cause.is_some() && fix.is_some(),
            }),

            ProcessedEvent::ServiceSilent {
                service,
                last_seen,
                silence_secs,
                last_sample,
            } => serde_json::json!({
                "kind": "service_silent",
                "service": service,
                "last_seen": last_seen,
                "silence_secs": silence_secs,
                "last_sample": last_sample,
            }),
        };

        HecEvent {
            time: self.event_time(),
            host: host.to_string(),
            source: self.source().to_string(),
            sourcetype: self.sourcetype().to_string(),
            index: index.map(str::to_string),
            event: payload,
        }
    }
}
