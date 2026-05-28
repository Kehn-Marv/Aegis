//! Lightweight, dependency-free identifier helpers.
//!
//! Aegis needs short, copy-pasteable IDs for incidents and decisions. We
//! don't need true UUID v4 guarantees — collision probability of ~1 in
//! 2^60 is more than enough for a local edge gateway. Using `blake3` over
//! a small seed (`monotonic_now()` + process counter) keeps us free of
//! the `uuid` crate while still giving us a 16-character alphanumeric
//! identifier that's safe to print, log, and use as a SQLite primary key.

use blake3::Hasher;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Return a 16-character lowercase hexadecimal identifier.
///
/// Two callers in the same process never get the same value because we
/// mix in a monotonic atomic counter. Across processes / restarts, the
/// SystemTime component is unique to nanosecond resolution.
pub fn short_uuid() -> String {
    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = Hasher::new();
    hasher.update(&now_nanos.to_le_bytes());
    hasher.update(&counter.to_le_bytes());
    let digest = hasher.finalize();
    let bytes = &digest.as_bytes()[..8];
    let mut out = String::with_capacity(16);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{:02x}", b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn returns_sixteen_hex_chars() {
        let id = short_uuid();
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn ids_are_unique_within_a_run() {
        let mut seen = HashSet::new();
        for _ in 0..1024 {
            assert!(seen.insert(short_uuid()));
        }
    }
}
