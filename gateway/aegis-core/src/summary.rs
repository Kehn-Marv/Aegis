//! Per-source accumulator that turns N routine-classified `Collapsed`
//! events into one rich `Summary` event per window.
//!
//! The dedup engine holds one accumulator per source string. When it would
//! normally emit a `Collapsed` event with `classification.label == "routine"`,
//! it instead calls `observe` and suppresses the emission. A periodic timer
//! in the dedup loop calls `drain` to produce `Summary` events that go
//! downstream like any other `ProcessedEvent`.
//!
//! Tradeoff: a routine signature that crash-looped *then* recovered will
//! be summarised away. We accept this because:
//!   1. The first occurrence of each signature is always forwarded raw
//!      (so the operator has incident context regardless of dedup/summary).
//!   2. An external AI agent can call `aegis.override` to bypass both
//!      dedup and summarization during an active investigation.

use crate::event::{ProcessedEvent, TopSig};
use std::collections::HashMap;

const TOP_N: usize = 5;

#[derive(Debug, Default, Clone)]
struct PerSignature {
    count: u64,
    sample: String,
}

#[derive(Debug, Default, Clone)]
pub struct SourceAccumulator {
    first_seen_unix: f64,
    last_seen_unix: f64,
    suppressed_lines: u64,
    signatures: HashMap<String, PerSignature>,
}

impl SourceAccumulator {
    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }

    pub fn observe(
        &mut self,
        signature: &str,
        count: u64,
        sample: &str,
        first_seen: f64,
        last_seen: f64,
    ) {
        if self.signatures.is_empty() {
            self.first_seen_unix = first_seen;
        } else {
            self.first_seen_unix = self.first_seen_unix.min(first_seen);
        }
        self.last_seen_unix = self.last_seen_unix.max(last_seen);
        self.suppressed_lines = self.suppressed_lines.saturating_add(count);
        let entry = self
            .signatures
            .entry(signature.to_string())
            .or_insert_with(|| PerSignature {
                count: 0,
                sample: sample.to_string(),
            });
        entry.count = entry.count.saturating_add(count);
    }

    pub fn drain_into_event(&mut self, source: String) -> ProcessedEvent {
        let unique_signatures = self.signatures.len() as u64;
        let mut all: Vec<(String, PerSignature)> = std::mem::take(&mut self.signatures)
            .into_iter()
            .collect();
        all.sort_by(|a, b| b.1.count.cmp(&a.1.count).then_with(|| a.0.cmp(&b.0)));
        let top_signatures: Vec<TopSig> = all
            .into_iter()
            .take(TOP_N)
            .map(|(signature, ps)| TopSig {
                signature,
                count: ps.count,
                sample: ps.sample,
            })
            .collect();
        let window_secs = (self.last_seen_unix - self.first_seen_unix).max(0.0);
        let ev = ProcessedEvent::Summary {
            source,
            window_secs,
            first_seen: self.first_seen_unix,
            last_seen: self.last_seen_unix,
            suppressed_lines: self.suppressed_lines,
            unique_signatures,
            top_signatures,
        };
        *self = Self::default();
        ev
    }
}

#[derive(Debug, Default)]
pub struct SummaryTable {
    by_source: HashMap<String, SourceAccumulator>,
}

impl SummaryTable {
    pub fn observe(
        &mut self,
        source: &str,
        signature: &str,
        count: u64,
        sample: &str,
        first_seen: f64,
        last_seen: f64,
    ) {
        self.by_source
            .entry(source.to_string())
            .or_default()
            .observe(signature, count, sample, first_seen, last_seen);
    }

    /// Flush every non-empty accumulator. Returns one event per source that
    /// had at least one observation since the last flush.
    pub fn drain_all(&mut self) -> Vec<ProcessedEvent> {
        let mut out = Vec::new();
        // Drain by key so we can move each accumulator independently.
        let sources: Vec<String> = self
            .by_source
            .iter()
            .filter(|(_, acc)| !acc.is_empty())
            .map(|(s, _)| s.clone())
            .collect();
        for source in sources {
            if let Some(acc) = self.by_source.get_mut(&source) {
                if !acc.is_empty() {
                    out.push(acc.drain_into_event(source));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_signatures_into_top_n() {
        let mut t = SummaryTable::default();
        for i in 0..10 {
            // Inject 10 distinct signatures, with sig "X" having the most volume.
            t.observe("svc", &format!("sig-{i}"), (i + 1) as u64, "sample", 1.0, 5.0);
        }
        t.observe("svc", "sig-X", 10_000, "boom", 0.5, 10.0);

        let events = t.drain_all();
        assert_eq!(events.len(), 1);
        match &events[0] {
            ProcessedEvent::Summary {
                source,
                suppressed_lines,
                unique_signatures,
                top_signatures,
                first_seen,
                last_seen,
                ..
            } => {
                assert_eq!(source, "svc");
                assert_eq!(*unique_signatures, 11);
                assert_eq!(*suppressed_lines, 10 * 11 / 2 + 10_000);
                assert_eq!(top_signatures.len(), TOP_N);
                assert_eq!(top_signatures[0].signature, "sig-X");
                assert_eq!(top_signatures[0].count, 10_000);
                assert_eq!(*first_seen, 0.5);
                assert_eq!(*last_seen, 10.0);
            }
            other => panic!("expected Summary, got {other:?}"),
        }
    }

    #[test]
    fn empty_table_drains_to_nothing() {
        let mut t = SummaryTable::default();
        assert!(t.drain_all().is_empty());
    }

    #[test]
    fn drain_resets_state() {
        let mut t = SummaryTable::default();
        t.observe("svc", "sig", 5, "x", 1.0, 2.0);
        let first = t.drain_all();
        assert_eq!(first.len(), 1);
        // Second drain should be empty until new observations land.
        let second = t.drain_all();
        assert!(second.is_empty());
    }
}
