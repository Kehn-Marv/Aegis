//! Shared control state used by the data plane, the MCP server, and the
//! REST API.
//!
//! Cheap to clone — internally an `Arc`. Hot fields are stored as atomics so
//! the data plane can read them without taking any locks; slower-changing
//! fields (latest decision card, incident memory count, current health
//! state) live behind small `Mutex`-guarded slots.

use crate::event::{HealthState, ProcessedEvent};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Public, JSON-serialisable status snapshot returned by the `status` MCP
/// tool and the `/api/status` REST endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GatewayStatus {
    pub uptime_secs: u64,
    pub online: bool,
    pub override_active: bool,
    pub diagnostic_active: bool,
    pub queue_depth: u64,
    pub events_in: u64,
    pub events_out: u64,
    pub dedup_savings_pct: f64,
    pub unique_signatures: u64,
    /// Current health colour (`green`, `orange`, `red`). Renders the badge
    /// the UI shows above the fold.
    pub state: String,
    /// Number of fingerprints currently in the incident-memory store.
    pub incidents_remembered: u64,
    /// The card the engineer should be looking at right now, if any.
    /// `null` means there's no active incident and no orange watch.
    pub decision: Option<ProcessedEvent>,
}

/// In-memory control surface. Cheap to clone (`Arc` inside).
#[derive(Clone)]
pub struct Control {
    inner: Arc<Inner>,
}

struct Inner {
    started_at: u64,
    online: AtomicBool,
    override_until_ms: AtomicU64,
    diagnostic_until_ms: AtomicU64,
    queue_depth: AtomicU64,
    events_in: AtomicU64,
    events_out: AtomicU64,
    unique_signatures: AtomicU64,
    incidents_remembered: AtomicU64,
    // Slower-changing fields. We don't `Mutex` the atomics above because
    // they're hammered on the hot path; these three are touched at most
    // a few times per second.
    state: Mutex<HealthState>,
    latest_decision: Mutex<Option<ProcessedEvent>>,
}

impl Default for Control {
    fn default() -> Self {
        Self::new()
    }
}

impl Control {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                started_at: now_secs(),
                online: AtomicBool::new(true),
                override_until_ms: AtomicU64::new(0),
                diagnostic_until_ms: AtomicU64::new(0),
                queue_depth: AtomicU64::new(0),
                events_in: AtomicU64::new(0),
                events_out: AtomicU64::new(0),
                unique_signatures: AtomicU64::new(0),
                incidents_remembered: AtomicU64::new(0),
                state: Mutex::new(HealthState::Green),
                latest_decision: Mutex::new(None),
            }),
        }
    }

    pub fn snapshot(&self) -> GatewayStatus {
        let events_in = self.inner.events_in.load(Ordering::Relaxed);
        let events_out = self.inner.events_out.load(Ordering::Relaxed);
        let dedup_savings_pct = if events_in == 0 {
            0.0
        } else {
            100.0 * (1.0 - (events_out as f64 / events_in as f64))
        };
        let state = self
            .inner
            .state
            .lock()
            .map(|g| *g)
            .unwrap_or(HealthState::Green);
        let decision = self
            .inner
            .latest_decision
            .lock()
            .ok()
            .and_then(|g| g.clone());
        GatewayStatus {
            uptime_secs: now_secs().saturating_sub(self.inner.started_at),
            online: self.inner.online.load(Ordering::Relaxed),
            override_active: self.override_active(),
            diagnostic_active: self.diagnostic_active(),
            queue_depth: self.inner.queue_depth.load(Ordering::Relaxed),
            events_in,
            events_out,
            dedup_savings_pct,
            unique_signatures: self.inner.unique_signatures.load(Ordering::Relaxed),
            state: state.as_str().to_string(),
            incidents_remembered: self.inner.incidents_remembered.load(Ordering::Relaxed),
            decision,
        }
    }

    pub fn set_online(&self, online: bool) {
        self.inner.online.store(online, Ordering::Relaxed);
    }

    pub fn enable_override(&self, duration_secs: u64) {
        let until = now_ms() + duration_secs.saturating_mul(1_000);
        self.inner.override_until_ms.store(until, Ordering::Relaxed);
    }

    pub fn enable_diagnostic(&self, duration_secs: u64) {
        let until = now_ms() + duration_secs.saturating_mul(1_000);
        self.inner
            .diagnostic_until_ms
            .store(until, Ordering::Relaxed);
    }

    pub fn override_active(&self) -> bool {
        self.inner.override_until_ms.load(Ordering::Relaxed) > now_ms()
    }

    pub fn diagnostic_active(&self) -> bool {
        self.inner.diagnostic_until_ms.load(Ordering::Relaxed) > now_ms()
    }

    pub fn reset(&self) {
        self.inner.queue_depth.store(0, Ordering::Relaxed);
        self.inner.events_in.store(0, Ordering::Relaxed);
        self.inner.events_out.store(0, Ordering::Relaxed);
        self.inner.unique_signatures.store(0, Ordering::Relaxed);
        self.inner.override_until_ms.store(0, Ordering::Relaxed);
        self.inner.diagnostic_until_ms.store(0, Ordering::Relaxed);
        if let Ok(mut s) = self.inner.state.lock() {
            *s = HealthState::Green;
        }
        if let Ok(mut d) = self.inner.latest_decision.lock() {
            *d = None;
        }
    }

    pub fn observe_in(&self, n: u64) {
        self.inner.events_in.fetch_add(n, Ordering::Relaxed);
    }

    pub fn observe_out(&self, n: u64) {
        self.inner.events_out.fetch_add(n, Ordering::Relaxed);
    }

    pub fn set_queue_depth(&self, n: u64) {
        self.inner.queue_depth.store(n, Ordering::Relaxed);
    }

    pub fn set_unique_signatures(&self, n: u64) {
        self.inner.unique_signatures.store(n, Ordering::Relaxed);
    }

    /// Update the gauge surfaced by `/api/status.incidents_remembered`.
    pub fn set_incidents_remembered(&self, n: u64) {
        self.inner
            .incidents_remembered
            .store(n, Ordering::Relaxed);
    }

    pub fn set_state(&self, state: HealthState) {
        if let Ok(mut s) = self.inner.state.lock() {
            *s = state;
        }
    }

    pub fn state(&self) -> HealthState {
        self.inner
            .state
            .lock()
            .map(|g| *g)
            .unwrap_or(HealthState::Green)
    }

    pub fn set_latest_decision(&self, ev: Option<ProcessedEvent>) {
        if let Ok(mut d) = self.inner.latest_decision.lock() {
            *d = ev;
        }
    }

    pub fn latest_decision(&self) -> Option<ProcessedEvent> {
        self.inner.latest_decision.lock().ok().and_then(|g| g.clone())
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
