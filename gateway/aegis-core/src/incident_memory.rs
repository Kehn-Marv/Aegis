//! Aegis's institutional memory.
//!
//! Every time Aegis detects a multi-service causal chain, it computes a
//! **fingerprint** — the set of involved signatures, the ordered service
//! chain, and the root cause — and stores it in a small SQLite database.
//!
//! When a new chain arrives, we hand it to [`Store::search_similar`] which
//! ranks past fingerprints by similarity and returns the top matches. Each
//! match carries the past cause + fix text an engineer filled in when
//! resolving the incident (if it was ever resolved).
//!
//! Similarity is a weighted blend designed to be **fast, local, and free**:
//!
//!   * **0.50 × Jaccard(signatures)** — same error patterns?
//!   * **0.30 × Jaccard(services)**   — same services involved?
//!   * **0.15 × chain-order LCS**     — same temporal order?
//!   * **0.05 × root-service match**  — bonus when the root cause matches.
//!
//! On a database with 10,000 incidents the search is still sub-millisecond
//! because we precompute and store the signature/service sets as
//! deterministically-ordered, comma-joined strings in SQLite. No embedding
//! model, no vector DB, no network call.

use crate::event::{CausalLink, IncidentMatch, ProcessedEvent};
use crate::id::short_uuid;
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::debug;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS incidents (
    id                    TEXT PRIMARY KEY,
    chain_id              TEXT NOT NULL,
    ts                    REAL NOT NULL,
    root_cause_service    TEXT NOT NULL,
    services_json         TEXT NOT NULL,
    signatures_json       TEXT NOT NULL,
    chain_json            TEXT NOT NULL,
    cause                 TEXT,
    fix                   TEXT,
    resolved_at           REAL,
    resolved_in_minutes   INTEGER
);
CREATE INDEX IF NOT EXISTS incidents_ts_idx     ON incidents(ts);
CREATE INDEX IF NOT EXISTS incidents_chain_idx  ON incidents(chain_id);
"#;

/// A fingerprint persisted to disk plus the optional resolution card.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fingerprint {
    pub id: String,
    pub chain_id: String,
    pub ts: f64,
    pub root_cause_service: String,
    pub services: Vec<String>,
    pub signatures: Vec<String>,
    pub chain: Vec<CausalLink>,
    pub cause: Option<String>,
    pub fix: Option<String>,
    pub resolved_at: Option<f64>,
    pub resolved_in_minutes: Option<i64>,
}

impl Fingerprint {
    pub fn services_set(&self) -> BTreeSet<&str> {
        self.services.iter().map(String::as_str).collect()
    }

    pub fn signatures_set(&self) -> BTreeSet<&str> {
        self.signatures.iter().map(String::as_str).collect()
    }

    pub fn is_resolved(&self) -> bool {
        self.cause.is_some() && self.fix.is_some()
    }

    pub fn to_event(&self) -> ProcessedEvent {
        ProcessedEvent::IncidentMemory {
            incident_id: self.id.clone(),
            chain_id: self.chain_id.clone(),
            ts: self.ts,
            root_cause_service: self.root_cause_service.clone(),
            services: self.services.clone(),
            cause: self.cause.clone(),
            fix: self.fix.clone(),
            resolved_at: self.resolved_at,
            resolved_in_minutes: self.resolved_in_minutes,
        }
    }
}

/// What an engineer fills in when an incident is resolved.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolutionCard {
    /// 1-2 sentences explaining the actual root cause in plain English.
    pub cause: String,
    /// 1-2 sentences explaining the fix that actually worked.
    pub fix: String,
}

/// Persistent incident-memory store backed by SQLite.
///
/// Cheap to clone — wraps the connection in `Arc<Mutex<…>>` so multiple
/// pipeline stages share one open handle.
#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path_ref = path.as_ref();
        if path_ref != Path::new(":memory:") {
            if let Some(parent) = path_ref.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("create incident dir {}", parent.display()))?;
                }
            }
        }
        let conn = Connection::open(path_ref)
            .with_context(|| format!("open incident sqlite at {}", path_ref.display()))?;
        conn.execute_batch(SCHEMA)
            .context("apply incident memory schema")?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "synchronous", "NORMAL").ok();
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Persist a fresh fingerprint generated from a `CausalChain` event.
    /// Returns the assigned incident id.
    pub fn record_chain(&self, event: &ProcessedEvent) -> Result<String> {
        let ProcessedEvent::CausalChain {
            chain_id,
            root_cause_service,
            chain,
            first_seen,
            ..
        } = event
        else {
            anyhow::bail!("record_chain requires a CausalChain event");
        };

        let fp = Fingerprint {
            id: short_uuid(),
            chain_id: chain_id.clone(),
            ts: *first_seen,
            root_cause_service: root_cause_service.clone(),
            services: chain.iter().map(|l| l.service.clone()).collect(),
            signatures: chain.iter().map(|l| l.signature.clone()).collect(),
            chain: chain.clone(),
            cause: None,
            fix: None,
            resolved_at: None,
            resolved_in_minutes: None,
        };
        self.insert(&fp)?;
        Ok(fp.id)
    }

    fn insert(&self, fp: &Fingerprint) -> Result<()> {
        let services_json = serde_json::to_string(&fp.services)?;
        let signatures_json = serde_json::to_string(&fp.signatures)?;
        let chain_json = serde_json::to_string(&fp.chain)?;
        let conn = self.conn.lock().expect("incident store poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO incidents
             (id, chain_id, ts, root_cause_service, services_json, signatures_json, chain_json,
              cause, fix, resolved_at, resolved_in_minutes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                fp.id,
                fp.chain_id,
                fp.ts,
                fp.root_cause_service,
                services_json,
                signatures_json,
                chain_json,
                fp.cause,
                fp.fix,
                fp.resolved_at,
                fp.resolved_in_minutes,
            ],
        )?;
        Ok(())
    }

    /// Attach a resolution card to an existing fingerprint. Returns the
    /// updated fingerprint when the id is known, `None` otherwise.
    pub fn resolve(&self, incident_id: &str, card: ResolutionCard, now_unix: f64) -> Result<Option<Fingerprint>> {
        let existing = self.get(incident_id)?;
        let Some(mut fp) = existing else {
            return Ok(None);
        };
        let resolved_in_minutes = ((now_unix - fp.ts) / 60.0).max(0.0).round() as i64;
        fp.cause = Some(card.cause);
        fp.fix = Some(card.fix);
        fp.resolved_at = Some(now_unix);
        fp.resolved_in_minutes = Some(resolved_in_minutes);
        self.insert(&fp)?;
        Ok(Some(fp))
    }

    pub fn get(&self, incident_id: &str) -> Result<Option<Fingerprint>> {
        let conn = self.conn.lock().expect("incident store poisoned");
        let row = conn
            .query_row(
                "SELECT id, chain_id, ts, root_cause_service, services_json,
                        signatures_json, chain_json, cause, fix, resolved_at, resolved_in_minutes
                 FROM incidents WHERE id = ?1",
                [incident_id],
                row_to_fingerprint,
            )
            .optional()?;
        Ok(row)
    }

    pub fn recent(&self, limit: i64) -> Result<Vec<Fingerprint>> {
        let conn = self.conn.lock().expect("incident store poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, chain_id, ts, root_cause_service, services_json,
                    signatures_json, chain_json, cause, fix, resolved_at, resolved_in_minutes
             FROM incidents ORDER BY ts DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], row_to_fingerprint)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Rank stored fingerprints by similarity to the given chain and return
    /// at most `top_n` matches above `min_similarity`.
    pub fn search_similar(
        &self,
        chain: &ProcessedEvent,
        top_n: usize,
        min_similarity: f32,
    ) -> Result<Vec<IncidentMatch>> {
        let ProcessedEvent::CausalChain {
            chain: links,
            root_cause_service,
            ..
        } = chain
        else {
            return Ok(Vec::new());
        };

        let query_services: HashSet<&str> = links.iter().map(|l| l.service.as_str()).collect();
        let query_signatures: HashSet<&str> = links.iter().map(|l| l.signature.as_str()).collect();
        let query_order: Vec<&str> = links.iter().map(|l| l.service.as_str()).collect();

        let stored = self.recent(2048)?;
        let mut scored: Vec<(f32, Fingerprint)> = Vec::new();
        for fp in stored {
            // Don't return the same chain back to itself when we're still
            // computing matches for a freshly recorded fingerprint.
            if let Some(self_chain_id) = chain_id_of(chain) {
                if fp.chain_id == self_chain_id {
                    continue;
                }
            }
            let similarity = similarity(
                &query_signatures,
                &query_services,
                &query_order,
                root_cause_service,
                &fp,
            );
            if similarity >= min_similarity {
                scored.push((similarity, fp));
            }
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_n);

        Ok(scored
            .into_iter()
            .map(|(sim, fp)| IncidentMatch {
                incident_id: fp.id.clone(),
                similarity: sim,
                past_ts: fp.ts,
                past_root_cause_service: fp.root_cause_service.clone(),
                past_cause: fp.cause.clone(),
                past_fix: fp.fix.clone(),
                past_resolved_in_minutes: fp.resolved_in_minutes,
            })
            .collect())
    }

    /// Approximate row count.
    pub fn count(&self) -> Result<u64> {
        let conn = self.conn.lock().expect("incident store poisoned");
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM incidents", [], |r| r.get(0))
            .unwrap_or(0);
        Ok(n as u64)
    }
}

fn row_to_fingerprint(row: &rusqlite::Row<'_>) -> rusqlite::Result<Fingerprint> {
    let services_json: String = row.get(4)?;
    let signatures_json: String = row.get(5)?;
    let chain_json: String = row.get(6)?;
    Ok(Fingerprint {
        id: row.get(0)?,
        chain_id: row.get(1)?,
        ts: row.get(2)?,
        root_cause_service: row.get(3)?,
        services: serde_json::from_str(&services_json).unwrap_or_default(),
        signatures: serde_json::from_str(&signatures_json).unwrap_or_default(),
        chain: serde_json::from_str(&chain_json).unwrap_or_default(),
        cause: row.get(7)?,
        fix: row.get(8)?,
        resolved_at: row.get(9)?,
        resolved_in_minutes: row.get(10)?,
    })
}

fn chain_id_of(event: &ProcessedEvent) -> Option<&str> {
    match event {
        ProcessedEvent::CausalChain { chain_id, .. } => Some(chain_id.as_str()),
        _ => None,
    }
}

fn jaccard(a: &HashSet<&str>, b: &BTreeSet<&str>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let mut inter = 0_usize;
    for x in a {
        if b.contains(x) {
            inter += 1;
        }
    }
    let union = a.len() + b.len() - inter;
    if union == 0 {
        0.0
    } else {
        inter as f32 / union as f32
    }
}

/// Longest-common-subsequence ratio between two service orders. Returns
/// a value in `[0, 1]` where `1` means identical ordered chains.
fn order_lcs_ratio(a: &[&str], b: &[String]) -> f32 {
    let n = a.len();
    let m = b.len();
    if n == 0 || m == 0 {
        return 0.0;
    }
    let mut dp = vec![vec![0_usize; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if a[i - 1] == b[j - 1].as_str() {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    let lcs = dp[n][m] as f32;
    lcs / n.max(m) as f32
}

fn similarity(
    query_sigs: &HashSet<&str>,
    query_services: &HashSet<&str>,
    query_order: &[&str],
    query_root: &str,
    fp: &Fingerprint,
) -> f32 {
    let stored_sigs = fp.signatures_set();
    let stored_services = fp.services_set();

    let mut score = 0.0;
    score += 0.50 * jaccard(query_sigs, &stored_sigs);
    score += 0.30 * jaccard(query_services, &stored_services);
    score += 0.15 * order_lcs_ratio(query_order, &fp.services);
    if fp.root_cause_service == query_root {
        score += 0.05;
    }
    debug!(
        stored_id = %fp.id,
        score = %score,
        "incident similarity"
    );
    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::CausalLink;

    fn make_chain(root: &str, services: &[&str], signatures: &[&str]) -> ProcessedEvent {
        let links: Vec<CausalLink> = services
            .iter()
            .zip(signatures.iter())
            .enumerate()
            .map(|(i, (svc, sig))| CausalLink {
                service: svc.to_string(),
                signature: sig.to_string(),
                ts: i as f64,
                ts_offset_secs: i as f64,
                sample: format!("ERROR {svc}: boom"),
            })
            .collect();
        ProcessedEvent::CausalChain {
            chain_id: short_uuid(),
            root_cause_service: root.to_string(),
            confidence: 0.9,
            chain: links,
            first_seen: 0.0,
            last_seen: services.len() as f64,
            suppressed_lines: 100,
        }
    }

    #[test]
    fn jaccard_basic() {
        let a: HashSet<&str> = ["x", "y", "z"].into_iter().collect();
        let b: BTreeSet<&str> = ["y", "z", "w"].into_iter().collect();
        let j = jaccard(&a, &b);
        // |{y,z}| = 2, |union| = 4 → 0.5
        assert!((j - 0.5).abs() < 1e-6);
    }

    #[test]
    fn order_lcs_identical() {
        let a: &[&str] = &["a", "b", "c"];
        let b: Vec<String> = a.iter().map(|s| s.to_string()).collect();
        assert!((order_lcs_ratio(a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn order_lcs_subsequence() {
        let a: &[&str] = &["a", "b", "c", "d"];
        let b: Vec<String> = ["a", "c"].iter().map(|s| s.to_string()).collect();
        // LCS is "a c" with length 2; max(n,m) is 4; ratio = 0.5
        let r = order_lcs_ratio(a, &b);
        assert!((r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn record_and_recall_round_trip() {
        let store = Store::open(":memory:").unwrap();
        let chain = make_chain("payment-api", &["payment-api", "checkout"], &["s1", "s2"]);
        let id = store.record_chain(&chain).unwrap();
        let fp = store.get(&id).unwrap().unwrap();
        assert_eq!(fp.root_cause_service, "payment-api");
        assert_eq!(fp.services.len(), 2);
        assert!(!fp.is_resolved());

        let resolved = store
            .resolve(
                &id,
                ResolutionCard {
                    cause: "DB pool exhausted".into(),
                    fix: "Reduced retry interval to 30s".into(),
                },
                fp.ts + 60.0,
            )
            .unwrap()
            .unwrap();
        assert!(resolved.is_resolved());
        assert_eq!(resolved.resolved_in_minutes, Some(1));
    }

    #[test]
    fn similar_search_ranks_better_overlap_higher() {
        let store = Store::open(":memory:").unwrap();

        let past_a = make_chain(
            "payment-api",
            &["payment-api", "checkout", "orders"],
            &["s1", "s2", "s3"],
        );
        let past_b = make_chain(
            "auth",
            &["auth", "session-cache"],
            &["sX", "sY"],
        );
        store.record_chain(&past_a).unwrap();
        store.record_chain(&past_b).unwrap();

        // Resolve so the matches carry cause/fix text downstream.
        let pa_id = store.recent(2).unwrap()[1].id.clone();
        let _ = store.resolve(
            &pa_id,
            ResolutionCard {
                cause: "DB pool exhausted".into(),
                fix: "Reduced retry interval".into(),
            },
            10.0,
        );

        let new_chain = make_chain(
            "payment-api",
            &["payment-api", "checkout"],
            &["s1", "s2"],
        );
        let matches = store.search_similar(&new_chain, 3, 0.1).unwrap();
        assert!(!matches.is_empty());
        // The matching past incident must come first.
        assert_eq!(matches[0].past_root_cause_service, "payment-api");
        assert!(matches[0].similarity > 0.5);
    }

    #[test]
    fn does_not_match_itself() {
        let store = Store::open(":memory:").unwrap();
        let chain = make_chain("a", &["a", "b"], &["s1", "s2"]);
        let _id = store.record_chain(&chain).unwrap();
        let matches = store.search_similar(&chain, 3, 0.0).unwrap();
        assert!(matches.is_empty(), "must not match its own chain_id");
    }
}
