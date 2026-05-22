//! Output sinks for processed events.
//!
//! Phase 1a ships a human-readable stderr sink so the demo works without
//! Splunk configured. Phase 1b will add the HEC sink with offline buffering.

use crate::event::ProcessedEvent;
use tokio::sync::mpsc;
use tracing::info;

/// Pretty-print processed events to stderr. Stops when `rx` closes.
pub async fn run_stderr(mut rx: mpsc::Receiver<ProcessedEvent>) -> anyhow::Result<()> {
    info!("stderr sink ready");
    while let Some(ev) = rx.recv().await {
        eprintln!("{}", format_event(&ev));
    }
    Ok(())
}

fn format_event(ev: &ProcessedEvent) -> String {
    match ev {
        ProcessedEvent::FirstOccurrence {
            signature,
            line,
            source,
            ..
        } => format!(
            "[FIRST  sig={} src={}] {}",
            &signature[..12.min(signature.len())],
            source,
            line
        ),
        ProcessedEvent::Collapsed {
            signature,
            count,
            window_secs,
            sample,
            ..
        } => format!(
            "[DEDUP  sig={} x{:>5} in {:>5.1}s] {}",
            &signature[..12.min(signature.len())],
            count,
            window_secs,
            sample
        ),
        ProcessedEvent::Raw { line, source, .. } => {
            format!("[RAW    src={}] {}", source, line)
        }
        ProcessedEvent::Summary {
            source,
            window_secs,
            suppressed_lines,
            unique_signatures,
            ..
        } => format!(
            "[SUMRY  src={} {} sigs in {:>5.1}s] suppressed={} routine lines",
            source, unique_signatures, window_secs, suppressed_lines
        ),
    }
}
