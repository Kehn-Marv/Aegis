//! TOML configuration for the Aegis daemon.

use serde::{Deserialize, Serialize};
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
    /// When true, `Collapsed` events whose AI classification is `"routine"`
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
    /// Address for the streamable-HTTP MCP server. Set to `None` (omit) to
    /// disable the HTTP server entirely; the daemon will then only expose
    /// MCP via stdio when launched with `--mcp-only`.
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
