//! Structural log-signature hashing.
//!
//! A signature is the hash of a log line with high-cardinality tokens
//! (numbers, hex IDs, UUIDs, RFC3339 timestamps, IP addresses, durations)
//! masked out. Two messages that differ only in those tokens collapse to
//! the same signature and can be deduplicated into a single metric event.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Signature(pub [u8; 16]);

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sig:{}", self)
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

static MASKERS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        // RFC3339 / ISO8601 timestamps
        (
            Regex::new(
                r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?",
            )
            .unwrap(),
            "<TS>",
        ),
        // UUIDs
        (
            Regex::new(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b").unwrap(),
            "<UUID>",
        ),
        // IPv4 addresses
        (Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(), "<IP>"),
        // Hex blobs (request ids, short hashes, pointer addresses, etc.).
        // 8+ chars catches CUIDs, truncated UUIDs, short request ids, and
        // hex memory addresses without eating regular short words.
        (Regex::new(r"\b[0-9a-fA-F]{8,}\b").unwrap(), "<HEX>"),
        // Durations with units
        (Regex::new(r"\b\d+(?:\.\d+)?(?:ns|us|ms|s|m|h)\b").unwrap(), "<DUR>"),
        // Bare numbers (do this last so it doesn't eat parts of the above)
        (Regex::new(r"\b\d+(?:\.\d+)?\b").unwrap(), "<N>"),
    ]
});

/// Mask high-cardinality tokens out of a log line so two structurally
/// identical messages collapse to the same string before hashing.
pub fn mask(line: &str) -> String {
    let mut out = line.to_string();
    for (re, repl) in MASKERS.iter() {
        out = re.replace_all(&out, *repl).into_owned();
    }
    out
}

/// Compute a 128-bit structural signature for a log line.
pub fn compute(line: &str) -> Signature {
    let masked = mask(line);
    let full = blake3::hash(masked.as_bytes());
    let mut out = [0u8; 16];
    out.copy_from_slice(&full.as_bytes()[..16]);
    Signature(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_lines_share_signature() {
        let a = "ERROR [2026-05-21T22:11:03Z] payment-service: connection refused to 10.0.4.12:5432";
        let b = "ERROR [2026-05-21T22:11:04Z] payment-service: connection refused to 10.0.4.12:5432";
        assert_eq!(compute(a), compute(b));
    }

    #[test]
    fn distinct_lines_diverge() {
        let a = "ERROR [2026-05-21T22:11:03Z] payment-service: connection refused to 10.0.4.12:5432";
        let b = "INFO  [2026-05-21T22:11:03Z] payment-service: connection established to 10.0.4.12:5432";
        assert_ne!(compute(a), compute(b));
    }

    #[test]
    fn masks_uuid_and_request_id() {
        let line = "req=550e8400-e29b-41d4-a716-446655440000 rid=deadbeef0123456789abcdef status=500";
        let masked = mask(line);
        assert!(masked.contains("<UUID>"));
        assert!(masked.contains("<HEX>"));
        assert!(masked.contains("<N>"));
    }
}
