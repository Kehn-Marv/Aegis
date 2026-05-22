//! Events that flow through the Aegis pipeline.

use crate::hec::HecEvent;
use crate::signature::Signature;
use serde::{Deserialize, Serialize};

/// Priority used by the SQLite queue. Lower number = drains first.
/// Anomaly evidence (raw error lines, override-mode passthrough) goes first;
/// dedup metrics are medium; future summary events go last.
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

/// What the dedup engine emits downstream toward the sink.
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
        /// AI classification for this signature, populated asynchronously by
        /// the sidecar when configured. May be `None` if the classifier was
        /// disabled, unreachable, or the window closed before it answered.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        classification: Option<Classification>,
    },
    /// Raw passthrough event (override mode is active).
    Raw {
        line: String,
        ts: f64,
        source: String,
    },
    /// Periodic aggregate of routine-classified traffic for a single source.
    /// Replaces N individual `Collapsed` events with one richer summary.
    Summary {
        source: String,
        window_secs: f64,
        first_seen: f64,
        last_seen: f64,
        /// Total raw lines absorbed (sum of `count` across all suppressed
        /// `Collapsed` events plus their first occurrences).
        suppressed_lines: u64,
        /// Distinct signatures seen in the window.
        unique_signatures: u64,
        /// Top-N signatures by count.
        top_signatures: Vec<TopSig>,
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
        }
    }

    /// Drain priority in the queue. Anomaly evidence drains first.
    pub fn priority(&self) -> i64 {
        match self {
            ProcessedEvent::FirstOccurrence { .. } | ProcessedEvent::Raw { .. } => PRIORITY_HIGH,
            ProcessedEvent::Collapsed { .. } => PRIORITY_MEDIUM,
            ProcessedEvent::Summary { .. } => PRIORITY_LOW,
        }
    }

    /// Wall-clock time on the event (unix epoch seconds, sub-second precision).
    pub fn event_time(&self) -> f64 {
        match self {
            ProcessedEvent::FirstOccurrence { ts, .. } | ProcessedEvent::Raw { ts, .. } => *ts,
            ProcessedEvent::Collapsed { last_seen, .. }
            | ProcessedEvent::Summary { last_seen, .. } => *last_seen,
        }
    }

    /// Source identifier passed through to HEC (`source` field).
    pub fn source(&self) -> &str {
        match self {
            ProcessedEvent::FirstOccurrence { source, .. }
            | ProcessedEvent::Raw { source, .. }
            | ProcessedEvent::Collapsed { source, .. }
            | ProcessedEvent::Summary { source, .. } => source,
        }
    }

    /// Build an HEC payload for this event.
    pub fn to_hec_event(&self, host: &str, index: Option<&str>) -> HecEvent {
        let payload = match self {
            ProcessedEvent::FirstOccurrence {
                signature, line, ..
            } => serde_json::json!({
                "kind": "first_occurrence",
                "signature": signature,
                "line": line,
            }),
            ProcessedEvent::Raw { line, .. } => serde_json::json!({
                "kind": "raw",
                "line": line,
            }),
            ProcessedEvent::Collapsed {
                signature,
                count,
                window_secs,
                first_seen,
                last_seen,
                sample,
                classification,
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
