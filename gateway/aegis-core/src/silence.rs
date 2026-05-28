//! Silent-service detector.
//!
//! The absence of events is itself a signal. When a service that's been
//! chatty falls quiet for longer than `silence_secs`, Aegis emits a
//! `ServiceSilent` event so operators don't only learn about an outage from
//! errors that never arrive.
//!
//! Implementation: an in-memory ledger of `service → (last_seen_unix, last_sample)`
//! updated on every `FirstOccurrence`, `Collapsed`, `Raw`, or `Summary`
//! event flowing past. A periodic sweep checks for services whose
//! `last_seen` is older than the threshold and emits a single
//! `ServiceSilent` per offence (suppressed afterwards until the service
//! talks again).

use crate::event::ProcessedEvent;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::debug;

#[derive(Clone, Debug)]
pub struct SilenceParams {
    /// How long a service has to be quiet before it's flagged.
    pub silence_secs: u64,
    /// How often the sweep runs. Smaller values catch silences faster
    /// but consume slightly more CPU.
    pub sweep_secs: u64,
}

impl Default for SilenceParams {
    fn default() -> Self {
        Self {
            silence_secs: 120,
            sweep_secs: 10,
        }
    }
}

#[derive(Debug, Clone)]
struct Heartbeat {
    last_seen: f64,
    last_sample: Option<String>,
    /// `true` once we've already fired a `ServiceSilent` for this idle period.
    /// Reset to `false` when the service talks again.
    flagged: bool,
}

/// Run the silence detector. Reads every `ProcessedEvent` from `in_rx`,
/// re-emits it on `out_tx`, and additionally emits `ServiceSilent` events
/// when a tracked service crosses the silence threshold.
pub async fn run(
    params: SilenceParams,
    mut in_rx: mpsc::Receiver<ProcessedEvent>,
    out_tx: mpsc::Sender<ProcessedEvent>,
) -> anyhow::Result<()> {
    let mut beats: HashMap<String, Heartbeat> = HashMap::new();
    let mut sweep = interval(Duration::from_secs(params.sweep_secs.max(1)));
    sweep.tick().await; // skip immediate fire

    loop {
        tokio::select! {
            biased;

            maybe = in_rx.recv() => {
                let Some(ev) = maybe else { break };
                if let Some((svc, ts, sample)) = service_and_ts(&ev) {
                    let beat = beats.entry(svc).or_insert(Heartbeat {
                        last_seen: 0.0,
                        last_sample: None,
                        flagged: false,
                    });
                    if ts > beat.last_seen {
                        beat.last_seen = ts;
                        beat.last_sample = sample;
                        beat.flagged = false;
                    }
                }
                if out_tx.send(ev).await.is_err() {
                    break;
                }
            }

            _ = sweep.tick() => {
                let now = now_unix_secs();
                let threshold = params.silence_secs as f64;
                for (svc, beat) in beats.iter_mut() {
                    if beat.last_seen <= 0.0 || beat.flagged {
                        continue;
                    }
                    let silence = now - beat.last_seen;
                    if silence < threshold {
                        continue;
                    }
                    debug!(service = %svc, silence_secs = %silence, "service has gone quiet");
                    let ev = ProcessedEvent::ServiceSilent {
                        service: svc.clone(),
                        last_seen: beat.last_seen,
                        silence_secs: silence,
                        last_sample: beat.last_sample.clone(),
                    };
                    if out_tx.send(ev).await.is_err() {
                        return Ok(());
                    }
                    beat.flagged = true;
                }
            }
        }
    }

    Ok(())
}

fn service_and_ts(ev: &ProcessedEvent) -> Option<(String, f64, Option<String>)> {
    match ev {
        ProcessedEvent::FirstOccurrence { service, ts, line, .. } => {
            Some((service.clone(), *ts, Some(line.clone())))
        }
        ProcessedEvent::Raw { service, ts, line, .. } => {
            Some((service.clone(), *ts, Some(line.clone())))
        }
        ProcessedEvent::Collapsed { service, last_seen, sample, .. } => {
            Some((service.clone(), *last_seen, Some(sample.clone())))
        }
        _ => None,
    }
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

    fn first(service: &str, ts: f64) -> ProcessedEvent {
        ProcessedEvent::FirstOccurrence {
            signature: "sig".into(),
            line: format!("ERROR {service}: hello"),
            ts,
            source: "tcp://x".into(),
            service: service.into(),
        }
    }

    #[tokio::test]
    async fn does_not_flag_a_chatty_service() {
        let (in_tx, in_rx) = mpsc::channel::<ProcessedEvent>(8);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(8);
        let params = SilenceParams { silence_secs: 1, sweep_secs: 1 };
        let task = tokio::spawn(async move { run(params, in_rx, out_tx).await });

        in_tx.send(first("svc", now_unix_secs())).await.unwrap();
        // Drain the pass-through
        let _ = tokio::time::timeout(Duration::from_millis(200), out_rx.recv()).await;
        drop(in_tx);
        task.await.unwrap().unwrap();
        // No silent event should have been seen.
        assert!(out_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn flags_a_service_that_stops() {
        let (in_tx, in_rx) = mpsc::channel::<ProcessedEvent>(8);
        let (out_tx, mut out_rx) = mpsc::channel::<ProcessedEvent>(8);
        let params = SilenceParams { silence_secs: 1, sweep_secs: 1 };
        let task = tokio::spawn(async move { run(params, in_rx, out_tx).await });

        // First-occurrence at ts=now-5s; the next sweep finds it stale.
        in_tx
            .send(first("svc", now_unix_secs() - 5.0))
            .await
            .unwrap();

        let mut silent = None;
        for _ in 0..3 {
            if let Ok(Some(ev)) =
                tokio::time::timeout(Duration::from_millis(1500), out_rx.recv()).await
            {
                if matches!(ev, ProcessedEvent::ServiceSilent { .. }) {
                    silent = Some(ev);
                    break;
                }
            } else {
                break;
            }
        }
        drop(in_tx);
        task.await.unwrap().unwrap();
        let silent = silent.expect("expected a silent event");
        match silent {
            ProcessedEvent::ServiceSilent { service, .. } => assert_eq!(service, "svc"),
            _ => unreachable!(),
        }
    }
}
