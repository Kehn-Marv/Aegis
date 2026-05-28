//! Output sinks for processed events.
//!
//! The stderr sink is what runs in demo mode (no Splunk configured). It
//! formats every `ProcessedEvent` variant as a short, human-readable line
//! so an operator can verify the pipeline at a glance without leaving
//! their terminal. The HEC sink (`hec_sink.rs`) does the same job for
//! production Splunk.

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
            service,
            ..
        } => format!(
            "[FIRST   svc={:<14} sig={}] {}",
            service,
            short_sig(signature),
            line
        ),

        ProcessedEvent::Collapsed {
            signature,
            count,
            window_secs,
            sample,
            service,
            ..
        } => format!(
            "[DEDUP   svc={:<14} sig={} x{:>5} in {:>5.1}s] {}",
            service,
            short_sig(signature),
            count,
            window_secs,
            sample
        ),

        ProcessedEvent::Raw { line, service, .. } => {
            format!("[RAW     svc={:<14}] {}", service, line)
        }

        ProcessedEvent::Summary {
            source,
            window_secs,
            suppressed_lines,
            unique_signatures,
            ..
        } => format!(
            "[SUMMARY src={} {} sigs in {:>5.1}s] suppressed={} routine lines",
            source, unique_signatures, window_secs, suppressed_lines
        ),

        ProcessedEvent::CausalChain {
            root_cause_service,
            chain,
            confidence,
            ..
        } => {
            let services: Vec<String> = chain
                .iter()
                .map(|l| format!("{} (+{:.1}s)", l.service, l.ts_offset_secs))
                .collect();
            format!(
                "[CHAIN   root={} conf={:.0}%] {}",
                root_cause_service,
                confidence * 100.0,
                services.join(" → ")
            )
        }

        ProcessedEvent::DecisionCard {
            state,
            root_cause_service,
            headline,
            similar_incidents,
            ..
        } => format!(
            "[DECIDE  state={} root={}] {} ({} similar past incident{})",
            state.as_str(),
            root_cause_service.as_deref().unwrap_or("-"),
            headline,
            similar_incidents.len(),
            if similar_incidents.len() == 1 { "" } else { "s" },
        ),

        ProcessedEvent::IncidentMemory {
            incident_id,
            root_cause_service,
            cause,
            ..
        } => format!(
            "[MEMORY  id={} root={}] {}",
            incident_id,
            root_cause_service,
            cause.as_deref().unwrap_or("(open — no resolution yet)")
        ),

        ProcessedEvent::ServiceSilent {
            service,
            silence_secs,
            last_sample,
            ..
        } => format!(
            "[SILENT  svc={:<14} quiet={:>5.0}s] last={}",
            service,
            silence_secs,
            last_sample.as_deref().unwrap_or("?")
        ),
    }
}

fn short_sig(sig: &str) -> &str {
    let max = 12.min(sig.len());
    &sig[..max]
}
