//! Shared control state used by both the data plane and the MCP server.
//!
//! The MCP server holds an `Arc<Control>` and mutates flags (override mode,
//! diagnostic mode, etc.); the ingest pipeline reads the same flags on each
//! batch. Anything the MCP server can mutate or inspect lives here.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Public, JSON-serialisable status snapshot returned by the `status` MCP tool.
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
