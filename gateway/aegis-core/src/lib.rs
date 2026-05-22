//! Aegis edge gateway core.
//!
//! Data plane: ingestion, structural signature hashing, dedup-to-metric,
//! routine-traffic summarization, the local priority queue, and the Splunk
//! HEC client. The MCP control plane (`aegis-mcp`) and the daemon binary
//! (`aegis-daemon`) compose this crate; this crate intentionally has no
//! dependency on either of them.

pub mod config;
pub mod control;
pub mod dedup;
pub mod event;
pub mod hec;
pub mod hec_sink;
pub mod ingest;
pub mod pipeline;
pub mod queue;
pub mod self_metrics;
pub mod sidecar;
pub mod signature;
pub mod sink;
pub mod summary;

pub use config::AegisConfig;
pub use control::{Control, GatewayStatus};
pub use event::{IngestLine, ProcessedEvent};
pub use hec::{HecClient, HecConfig};
pub use queue::Queue;
pub use sidecar::SidecarClient;
pub use signature::Signature;
