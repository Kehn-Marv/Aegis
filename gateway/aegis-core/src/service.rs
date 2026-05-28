//! Extract a human-friendly service name from a log line.
//!
//! Aegis's causal-chain engine groups events by *which service produced them*.
//! That means we need a stable, low-cost way to pull a service tag out of an
//! unstructured log line. We try a small, ordered list of regex patterns —
//! the patterns are intentionally conservative: a false miss is fine (we
//! fall back to the ingest source), but a false match would pollute the
//! causal graph.
//!
//! Recognised shapes (in order):
//!   1. JSON: `"service":"payment-api"` or `"service_name":"…"`
//!   2. Structured prefix: `LEVEL service-name: …` (e.g. `ERROR payment-api: …`)
//!   3. Bracketed: `[service-name] …`
//!   4. Suffix: `… (svc=payment-api)` or `service=payment-api`
//!
//! Anything that doesn't match falls back to the ingest source identifier
//! (e.g. `tcp://127.0.0.1:5140`). For multi-tenant edges that want a
//! deterministic mapping, `extract_with_hint` lets the caller pin a service
//! name to a source (typically from config).

use once_cell::sync::Lazy;
use regex::Regex;

/// Common log levels we ignore when pulling out a `LEVEL service: …` shape.
const LEVELS: &[&str] = &[
    "TRACE", "DEBUG", "INFO", "WARN", "WARNING", "ERROR", "FATAL", "PANIC",
];

static RE_JSON_SERVICE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""service(?:_name)?"\s*:\s*"([A-Za-z0-9_.\-]{2,64})""#).unwrap()
});

static RE_KV_SERVICE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:svc|service)=([A-Za-z0-9_.\-]{2,64})\b").unwrap()
});

static RE_LEVEL_SERVICE: Lazy<Regex> = Lazy::new(|| {
    // Matches `LEVEL service-name:` or `LEVEL [timestamp] service-name:`.
    // We don't anchor the timestamp pattern strictly — `mask()` will have
    // already masked it; here we just hop over the optional `[…]`.
    Regex::new(
        r"^[A-Z]+\s+(?:\[[^\]]+\]\s+)?([a-zA-Z][A-Za-z0-9_.\-]{1,63})\s*:",
    )
    .unwrap()
});

static RE_BRACKETED: Lazy<Regex> = Lazy::new(|| {
    // `[service-name] rest`. Avoid matching `[ERROR]` or `[2026-…]` by
    // requiring the inner token to contain a letter and not be a known level.
    Regex::new(r"^\[([A-Za-z][A-Za-z0-9_.\-]{1,63})\]").unwrap()
});

/// Extract a service name from a log line, falling back to `source`.
///
/// The result is always non-empty: when nothing matches, it returns the
/// trimmed source string (or the literal `"unknown"` if the source itself
/// is empty).
pub fn extract(line: &str, source: &str) -> String {
    if let Some(name) = try_extract(line) {
        return name;
    }
    fallback(source)
}

/// True when the line looks like a *continuation* of the previous log
/// entry — typically a stack-trace frame (`  at db::Pool::checkout`) or a
/// `caused by: …` chain.
///
/// Multiline logs are the norm, not the exception, and binding stack
/// frames to the parent service is much more useful than treating each
/// frame as if it came from "tcp://127.0.0.1:54231".
pub fn is_continuation(line: &str) -> bool {
    // Leading whitespace is the single strongest signal (almost every
    // multiline log family indents continuation lines).
    if line.starts_with(' ') || line.starts_with('\t') {
        return true;
    }
    // Catch un-indented "caused by:" and a few common ORM/runtime-line
    // forms that some loggers don't indent.
    let lower = line.to_ascii_lowercase();
    lower.starts_with("caused by:")
        || lower.starts_with("at ")
        || lower.starts_with("traceback")
        || lower.starts_with("--> ")
}

/// Like [`extract`], but if `line` looks like a continuation of an
/// earlier entry and `fallback_service` is provided, the fallback wins.
/// Lets multi-line log records (stack traces, Java/Python tracebacks) be
/// attributed to the service that opened the trace.
pub fn extract_with_continuation(
    line: &str,
    source: &str,
    last_service_for_source: Option<&str>,
) -> String {
    if is_continuation(line) {
        if let Some(prev) = last_service_for_source {
            if !prev.is_empty() {
                return prev.to_string();
            }
        }
    }
    extract(line, source)
}

/// Same as [`extract`], but a caller-provided hint wins over inference.
///
/// Used when an operator wants to pin a source (e.g. a regional gateway
/// listening on a specific port) to a known service name regardless of
/// what the log content looks like.
pub fn extract_with_hint(line: &str, source: &str, hint: Option<&str>) -> String {
    if let Some(h) = hint {
        let trimmed = h.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    extract(line, source)
}

/// Full-featured extraction that combines a config hint with last-seen
/// service propagation for continuation lines. Used by the dedup engine.
pub fn extract_full(
    line: &str,
    source: &str,
    hint: Option<&str>,
    last_service_for_source: Option<&str>,
) -> String {
    if let Some(h) = hint {
        let trimmed = h.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    extract_with_continuation(line, source, last_service_for_source)
}

fn try_extract(line: &str) -> Option<String> {
    let trimmed = line.trim_start();

    if let Some(c) = RE_JSON_SERVICE.captures(trimmed) {
        return Some(c[1].to_string());
    }
    if let Some(c) = RE_KV_SERVICE.captures(trimmed) {
        return Some(c[1].to_string());
    }
    if let Some(c) = RE_LEVEL_SERVICE.captures(trimmed) {
        let candidate = c[1].to_string();
        // Avoid `ERROR ERROR:` style false positives.
        if !LEVELS.contains(&candidate.to_uppercase().as_str()) {
            return Some(candidate);
        }
    }
    if let Some(c) = RE_BRACKETED.captures(trimmed) {
        let candidate = c[1].to_string();
        if !LEVELS.contains(&candidate.to_uppercase().as_str()) {
            return Some(candidate);
        }
    }
    None
}

fn fallback(source: &str) -> String {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    // `tcp://127.0.0.1:54231` → keep the scheme + host:port. Already a fine
    // service identifier for demos; in production an operator would
    // normally configure `source_to_service` hints.
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_prefix() {
        assert_eq!(
            extract("ERROR payment-api: connection refused", "tcp://x"),
            "payment-api"
        );
    }

    #[test]
    fn level_with_timestamp_prefix() {
        assert_eq!(
            extract("ERROR [2026-05-21T12:00:00Z] order-svc: boom", "tcp://x"),
            "order-svc"
        );
    }

    #[test]
    fn json_service_field() {
        assert_eq!(
            extract(r#"{"service":"checkout","msg":"oops"}"#, "tcp://x"),
            "checkout"
        );
    }

    #[test]
    fn kv_service_field() {
        assert_eq!(
            extract("rid=abc svc=ledger-api status=500", "tcp://x"),
            "ledger-api"
        );
    }

    #[test]
    fn bracketed_service() {
        assert_eq!(
            extract("[fulfilment] picking up order #42", "tcp://x"),
            "fulfilment"
        );
    }

    #[test]
    fn falls_back_to_source() {
        assert_eq!(
            extract("just a noisy line with no service tag", "tcp://1.2.3.4:5140"),
            "tcp://1.2.3.4:5140"
        );
    }

    #[test]
    fn hint_wins() {
        assert_eq!(
            extract_with_hint("ERROR payment-api: oops", "tcp://x", Some("us-east-payment")),
            "us-east-payment"
        );
    }

    #[test]
    fn ignores_level_after_brackets() {
        // `[ERROR]` is a level, not a service.
        assert_eq!(
            extract("[ERROR] something blew up", "tcp://x"),
            "tcp://x"
        );
    }

    #[test]
    fn unknown_when_source_empty() {
        assert_eq!(extract("noise", ""), "unknown");
    }

    #[test]
    fn detects_continuation_lines() {
        assert!(is_continuation("  at db::Pool::checkout (db.rs:142)"));
        assert!(is_continuation("\tat handlers::charge (handlers.rs:88)"));
        assert!(is_continuation("caused by: io::Error: ConnectionRefused"));
        assert!(is_continuation("Caused by: java.sql.SQLException"));
        assert!(is_continuation("Traceback (most recent call last):"));
        assert!(!is_continuation("ERROR payment-api: connection refused"));
        assert!(!is_continuation("INFO 200 OK"));
    }

    #[test]
    fn continuation_inherits_last_service() {
        let svc = extract_with_continuation(
            "  at db::Pool::checkout (db.rs:142)",
            "tcp://127.0.0.1:5140",
            Some("payment-api"),
        );
        assert_eq!(svc, "payment-api");
    }

    #[test]
    fn continuation_without_last_service_falls_back() {
        let svc = extract_with_continuation(
            "  at db::Pool::checkout (db.rs:142)",
            "tcp://x",
            None,
        );
        assert_eq!(svc, "tcp://x");
    }

    #[test]
    fn extract_full_prioritises_hint_then_inference_then_inheritance() {
        // Hint wins
        let s = extract_full("ERROR payment-api: x", "tcp://x", Some("us-east-payment"), Some("ignored"));
        assert_eq!(s, "us-east-payment");
        // No hint → inference wins on a regular line
        let s = extract_full("ERROR payment-api: x", "tcp://x", None, Some("ignored"));
        assert_eq!(s, "payment-api");
        // No hint, continuation → inheritance wins
        let s = extract_full("  at db::checkout", "tcp://x", None, Some("payment-api"));
        assert_eq!(s, "payment-api");
    }
}
