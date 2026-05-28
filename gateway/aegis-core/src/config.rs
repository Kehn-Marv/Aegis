//! TOML configuration for the Aegis daemon.
//!
//! Every section here has a sensible default — you can run the daemon
//! against an empty file. The example file (`configs/aegis.example.toml`)
//! documents what each knob does in human language.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::hec::HecConfig;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AegisConfig {
    #[serde(default)]
    pub hec: Option<HecConfig>,

    #[serde(default)]
    pub ingest: IngestConfig,

    #[serde(default)]
    pub dedup: DedupConfig,

    #[serde(default)]
    pub summary: SummaryConfig,

    #[serde(default)]
    pub queue: QueueConfig,

    #[serde(default)]
    pub sidecar: SidecarConfig,

    #[serde(default)]
    pub self_metrics: SelfMetricsConfig,

    #[serde(default)]
    pub mcp: McpConfig,

    #[serde(default)]
    pub causal: CausalConfig,

    #[serde(default)]
    pub memory: MemoryConfig,

    #[serde(default)]
    pub decision: DecisionConfig,

    #[serde(default)]
    pub silence: SilenceConfig,

    /// Optional pin: `source_name → service_name`. When set, lines arriving
    /// from `source_name` are tagged with `service_name` regardless of the
    /// service inference rules.
    #[serde(default)]
    pub source_to_service: HashMap<String, String>,

    /// Optional `service → one-line business impact text` map. Surfaces in
    /// the decision card so engineers see "this is why it matters" without
    /// hunting for context.
    #[serde(default)]
    pub services: HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IngestConfig {
    #[serde(default = "default_tcp_listen")]
    pub tcp_listen: Option<String>,
    #[serde(default)]
    pub udp_listen: Option<String>,
    #[serde(default)]
    pub tail_files: Vec<String>,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            tcp_listen: default_tcp_listen(),
            udp_listen: None,
            tail_files: Vec::new(),
        }
    }
}

fn default_tcp_listen() -> Option<String> {
    Some("127.0.0.1:5140".to_string())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DedupConfig {
    #[serde(default = "default_window_secs")]
    pub window_secs: u64,
    #[serde(default = "default_max_open")]
    pub max_open_signatures: usize,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            window_secs: default_window_secs(),
            max_open_signatures: default_max_open(),
        }
    }
}

fn default_window_secs() -> u64 {
    30
}
fn default_max_open() -> usize {
    4096
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SummaryConfig {
    /// When true, `Collapsed` events whose AI classification is `routine`
    /// are suppressed and rolled into periodic `Summary` events instead.
    /// Has no effect without an AI sidecar to provide classifications.
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_flush_secs")]
    pub flush_secs: u64,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            batch_size: default_batch_size(),
            flush_secs: default_flush_secs(),
        }
    }
}

fn default_batch_size() -> usize {
    500
}
fn default_flush_secs() -> u64 {
    30
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueueConfig {
    #[serde(default = "default_queue_path")]
    pub path: String,
    #[serde(default = "default_max_disk")]
    pub max_disk_bytes: u64,
    #[serde(default = "default_true")]
    pub priority_anomaly_first: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            path: default_queue_path(),
            max_disk_bytes: default_max_disk(),
            priority_anomaly_first: true,
        }
    }
}

fn default_queue_path() -> String {
    "data/aegis-queue.sqlite".into()
}
fn default_max_disk() -> u64 {
    1_073_741_824
}
fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SidecarConfig {
    #[serde(default = "default_sidecar_url")]
    pub url: String,
    #[serde(default)]
    pub enabled: bool,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            url: default_sidecar_url(),
            enabled: false,
        }
    }
}

fn default_sidecar_url() -> String {
    "http://127.0.0.1:8765".into()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SelfMetricsConfig {
    #[serde(default = "default_self_source")]
    pub source: String,
    #[serde(default = "default_self_sourcetype")]
    pub sourcetype: String,
    #[serde(default = "default_self_flush")]
    pub flush_secs: u64,
}

impl Default for SelfMetricsConfig {
    fn default() -> Self {
        Self {
            source: default_self_source(),
            sourcetype: default_self_sourcetype(),
            flush_secs: default_self_flush(),
        }
    }
}

fn default_self_source() -> String {
    "aegis:self".into()
}
fn default_self_sourcetype() -> String {
    "aegis:selfmetric".into()
}
fn default_self_flush() -> u64 {
    15
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpConfig {
    /// Address for the streamable-HTTP MCP server. Set to `None` (omit)
    /// to disable the HTTP server entirely; the daemon will then only
    /// expose MCP via stdio when launched with `--mcp-only`.
    #[serde(default = "default_mcp_listen")]
    pub http_listen: Option<String>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            http_listen: default_mcp_listen(),
        }
    }
}

fn default_mcp_listen() -> Option<String> {
    Some("127.0.0.1:7321".to_string())
}

/// Causal chain detection knobs. See `gateway/aegis-core/src/causal.rs`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CausalConfig {
    #[serde(default = "default_causal_window")]
    pub window_secs: u64,
    #[serde(default = "default_causal_min_services")]
    pub min_services: usize,
    #[serde(default = "default_causal_cooldown")]
    pub cooldown_secs: u64,
}

impl Default for CausalConfig {
    fn default() -> Self {
        Self {
            window_secs: default_causal_window(),
            min_services: default_causal_min_services(),
            cooldown_secs: default_causal_cooldown(),
        }
    }
}

fn default_causal_window() -> u64 {
    60
}
fn default_causal_min_services() -> usize {
    3
}
fn default_causal_cooldown() -> u64 {
    300
}

/// Incident memory store knobs. The store path is `<queue_dir>/incidents.sqlite`
/// by default — i.e. the same `data/` folder the queue lives in.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryConfig {
    #[serde(default = "default_memory_path")]
    pub path: String,
    #[serde(default = "default_memory_top_n")]
    pub top_matches: usize,
    #[serde(default = "default_memory_min_similarity")]
    pub min_similarity: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            path: default_memory_path(),
            top_matches: default_memory_top_n(),
            min_similarity: default_memory_min_similarity(),
        }
    }
}

fn default_memory_path() -> String {
    "data/aegis-incidents.sqlite".into()
}
fn default_memory_top_n() -> usize {
    3
}
fn default_memory_min_similarity() -> f32 {
    0.25
}

/// Decision card synthesiser knobs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecisionConfig {
    #[serde(default = "default_idle_green_secs")]
    pub idle_to_green_secs: u64,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            idle_to_green_secs: default_idle_green_secs(),
        }
    }
}

fn default_idle_green_secs() -> u64 {
    300
}

/// Silent-service detector knobs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SilenceConfig {
    #[serde(default = "default_silence_secs")]
    pub silence_secs: u64,
    #[serde(default = "default_silence_sweep")]
    pub sweep_secs: u64,
    /// Set to `false` to disable the detector entirely. Useful for batch
    /// pipelines whose services come and go on schedule.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for SilenceConfig {
    fn default() -> Self {
        Self {
            silence_secs: default_silence_secs(),
            sweep_secs: default_silence_sweep(),
            enabled: true,
        }
    }
}

fn default_silence_secs() -> u64 {
    120
}
fn default_silence_sweep() -> u64 {
    10
}

impl AegisConfig {
    /// Load configuration from a TOML file. Returns `Default::default()` if
    /// the file does not exist (useful for first-run dev experience).
    pub fn load_or_default(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&text)?;
        Ok(cfg)
    }
}
