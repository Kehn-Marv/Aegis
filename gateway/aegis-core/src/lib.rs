//! Aegis edge gateway — core library.
//!
//! Four pillars, all running in a single Rust daemon:
//!
//!   1. **Noise gate** (`signature`, `dedup`, `summary`) — collapse repeating
//!      log lines into a single metric event per window, keep the first
//!      occurrence raw so context is never lost.
//!   2. **Causal chain** (`causal`) — when multiple services start failing
//!      inside the same window, identify who broke first.
//!   3. **Incident memory** (`incident_memory`) — fingerprint every chain,
//!      remember the cause + fix the engineer entered last time, surface
//!      similar past incidents on the next occurrence in sub-millisecond
//!      time using a small SQLite store.
//!   4. **Decision card** (`decision`) — turn the raw signals into one
//!      focused recommendation card the engineer reads, with state
//!      (green/orange/red), root cause, similar past incidents, business
//!      impact, and a single concrete next step.
//!
//! Plus a silent-service detector (`silence`) so absent services are
//! noticed before they cause downstream damage.
//!
//! The MCP control plane (`aegis-mcp`) and the daemon binary
//! (`aegis-daemon`) compose this crate; this crate intentionally has no
//! dependency on either of them.

pub mod causal;
pub mod config;
pub mod control;
pub mod decision;
pub mod dedup;
pub mod event;
pub mod hec;
pub mod hec_sink;
pub mod id;
pub mod incident_memory;
pub mod ingest;
pub mod pipeline;
pub mod queue;
pub mod self_metrics;
pub mod service;
pub mod service_catalog;
pub mod sidecar;
pub mod signature;
pub mod silence;
pub mod sink;
pub mod summary;

pub use config::AegisConfig;
pub use control::{Control, GatewayStatus};
pub use event::{
    CausalLink, Classification, HealthState, IncidentMatch, IngestLine, ProcessedEvent,
};
pub use hec::{HecClient, HecConfig};
pub use incident_memory::{Fingerprint, ResolutionCard, Store as IncidentStore};
pub use queue::Queue;
pub use service_catalog::ServiceCatalog;
pub use sidecar::SidecarClient;
pub use signature::Signature;
