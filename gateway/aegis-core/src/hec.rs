//! Minimal Splunk HTTP Event Collector client.
//!
//! Aegis pushes three distinct event shapes to HEC:
//!   * `sourcetype = aegis:raw`     — first-occurrence raw logs and override-mode streams
//!   * `sourcetype = aegis:metric`  — dedup metric events (`{signature, count, window, ...}`)
//!   * `sourcetype = aegis:summary` — routine-traffic batch summaries
//!
//! A fourth dedicated source carries Aegis's own self-metrics for the AI
//! Agent Monitoring dashboard:
//!   * `sourcetype = aegis:selfmetric`
//!
//! This is intentionally a thin wrapper; batching, retries, and offline
//! buffering live in the queue layer.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HecConfig {
    /// Full HEC endpoint, e.g. `https://splunk.example.com:8088/services/collector/event`.
    pub endpoint: String,
    /// HEC token (the `Authorization: Splunk <token>` value).
    pub token: String,
    /// Default index for events that don't specify one.
    #[serde(default)]
    pub index: Option<String>,
    /// Default host field; defaults to the machine hostname at runtime.
    #[serde(default)]
    pub host: Option<String>,
    /// Request timeout, default 10s.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Whether to verify TLS certificates (off for local dev).
    #[serde(default = "default_verify")]
    pub verify_tls: bool,
}

fn default_timeout() -> u64 {
    10
}
fn default_verify() -> bool {
    true
}

#[derive(Clone, Debug, Serialize)]
pub struct HecEvent {
    pub time: f64,
    pub host: String,
    pub source: String,
    pub sourcetype: String,
    pub index: Option<String>,
    pub event: serde_json::Value,
}

#[derive(Clone)]
pub struct HecClient {
    cfg: HecConfig,
    http: reqwest::Client,
}

impl HecClient {
    pub fn new(cfg: HecConfig) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .danger_accept_invalid_certs(!cfg.verify_tls)
            .build()?;
        Ok(Self { cfg, http })
    }

    pub fn config(&self) -> &HecConfig {
        &self.cfg
    }

    /// Send a batch of events to HEC. HEC accepts newline-delimited JSON.
    pub async fn send(&self, events: &[HecEvent]) -> anyhow::Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        let mut body = String::with_capacity(events.len() * 256);
        for e in events {
            body.push_str(&serde_json::to_string(e)?);
            body.push('\n');
        }
        let resp = self
            .http
            .post(&self.cfg.endpoint)
            .header("Authorization", format!("Splunk {}", self.cfg.token))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HEC rejected events: {} — {}", status, body);
        }
        Ok(())
    }
}
