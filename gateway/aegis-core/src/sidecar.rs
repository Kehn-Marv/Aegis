//! HTTP client for the Aegis AI sidecar (Python FastAPI service).
//!
//! Methods correspond 1:1 to sidecar endpoints. All calls are bounded by
//! the configured per-request timeout; the gateway treats sidecar errors
//! as soft failures (the data plane keeps moving with hash-based dedup).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct SidecarClient {
    base_url: String,
    http: reqwest::Client,
}

impl SidecarClient {
    pub fn new(base_url: impl Into<String>, timeout: Duration) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("build sidecar http client")?;
        Ok(Self {
            base_url: base_url.into(),
            http,
        })
    }

    pub async fn health(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url.trim_end_matches('/'));
        let resp = self.http.get(url).send().await?;
        Ok(resp.status().is_success())
    }

    pub async fn info(&self) -> Result<SidecarInfo> {
        let url = format!("{}/info", self.base_url.trim_end_matches('/'));
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn embed(&self, lines: &[String]) -> Result<EmbedResponse> {
        let url = format!("{}/embed", self.base_url.trim_end_matches('/'));
        let body = EmbedRequest {
            lines: lines.to_vec(),
        };
        let resp = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn cluster(
        &self,
        embeddings: &[Vec<f32>],
        k: Option<usize>,
    ) -> Result<ClusterResponse> {
        let url = format!("{}/cluster", self.base_url.trim_end_matches('/'));
        let body = ClusterRequest {
            embeddings: embeddings.to_vec(),
            k,
        };
        let resp = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn classify(&self, line: &str) -> Result<ClassifyResponse> {
        let url = format!("{}/classify", self.base_url.trim_end_matches('/'));
        let body = ClassifyRequest {
            line: line.to_string(),
        };
        let resp = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarInfo {
    pub embedding_model: String,
    pub embedding_dim: u32,
    pub embedding_fallback: bool,
    pub hosted_model_configured: bool,
    pub hosted_model_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct EmbedRequest {
    lines: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedResponse {
    pub dim: u32,
    pub embeddings: Vec<Vec<f32>>,
    pub model: String,
    pub fallback: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ClusterRequest {
    embeddings: Vec<Vec<f32>>,
    k: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClusterResponse {
    pub labels: Vec<i32>,
    pub n_clusters: u32,
}

#[derive(Debug, Clone, Serialize)]
struct ClassifyRequest {
    line: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClassifyResponse {
    pub label: String,
    pub confidence: f32,
    pub strategy: String,
    pub latency_ms: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_response_round_trips() {
        let json = r#"{"label":"anomaly","confidence":0.91,"strategy":"embedding_distance","latency_ms":12.4}"#;
        let parsed: ClassifyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.label, "anomaly");
        assert_eq!(parsed.strategy, "embedding_distance");
        assert!((parsed.confidence - 0.91).abs() < 1e-6);
    }

    #[test]
    fn embed_response_parses_fallback_flag() {
        let json = r#"{
            "dim": 32,
            "embeddings": [[0.1, 0.2]],
            "model": "fallback",
            "fallback": true
        }"#;
        let parsed: EmbedResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.fallback);
        assert_eq!(parsed.dim, 32);
    }

    #[test]
    fn cluster_response_parses() {
        let json = r#"{"labels":[0,1,0,2],"n_clusters":3}"#;
        let parsed: ClusterResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.labels, vec![0, 1, 0, 2]);
        assert_eq!(parsed.n_clusters, 3);
    }
}
