//! Local SQLite-backed priority queue.
//!
//! All ProcessedEvents flow through this queue between the dedup engine and
//! the HEC sink. The queue:
//!   * persists across restarts (offline-first resilience),
//!   * drains highest-priority entries first (anomalies before metrics),
//!   * gives the rest of the pipeline a single, uniform shape regardless of
//!     whether HEC is reachable.
//!
//! Synchronous rusqlite calls run inside `tokio::task::spawn_blocking`. The
//! connection is wrapped in `std::sync::Mutex` (not the tokio one) so it can
//! be locked from inside a blocking task without panicking.

use crate::event::ProcessedEvent;
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS queue (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    priority     INTEGER NOT NULL,
    sourcetype   TEXT    NOT NULL,
    payload      TEXT    NOT NULL,
    enqueued_at  REAL    NOT NULL
);
CREATE INDEX IF NOT EXISTS queue_priority_idx ON queue(priority, id);
"#;

#[derive(Clone)]
pub struct Queue {
    conn: Arc<Mutex<Connection>>,
    max_disk_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct QueueItem {
    pub id: i64,
    pub event: ProcessedEvent,
}

impl Queue {
    /// Open (or create) a queue at `path`. Use `":memory:"` for tests.
    pub fn open(path: impl AsRef<Path>, max_disk_bytes: u64) -> Result<Self> {
        let path_ref = path.as_ref();
        if path_ref != Path::new(":memory:") {
            if let Some(parent) = path_ref.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("create queue dir {}", parent.display()))?;
                }
            }
        }
        let conn = Connection::open(path_ref)
            .with_context(|| format!("open sqlite at {}", path_ref.display()))?;
        conn.execute_batch(SCHEMA).context("apply queue schema")?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "synchronous", "NORMAL").ok();
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            max_disk_bytes,
        })
    }

    /// Enqueue a single event. Drops the oldest low-priority entry when the
    /// queue exceeds `max_disk_bytes` (best-effort backpressure).
    pub async fn enqueue(&self, event: &ProcessedEvent) -> Result<()> {
        let payload = serde_json::to_string(event)?;
        let priority = event.priority();
        let sourcetype = event.sourcetype().to_string();
        let now = unix_secs_f64();
        let conn = self.conn.clone();
        let max_bytes = self.max_disk_bytes;
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = conn.lock().expect("queue mutex poisoned");
            ensure_capacity(&mut conn, max_bytes)?;
            conn.execute(
                "INSERT INTO queue (priority, sourcetype, payload, enqueued_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![priority, sourcetype, payload, now],
            )?;
            Ok(())
        })
        .await??;
        Ok(())
    }

    /// Peek up to `limit` items in drain order (priority asc, id asc) without
    /// removing them. Callers ack the returned ids after a successful send.
    pub async fn peek_batch(&self, limit: usize) -> Result<Vec<QueueItem>> {
        let conn = self.conn.clone();
        let items = tokio::task::spawn_blocking(move || -> Result<Vec<QueueItem>> {
            let conn = conn.lock().expect("queue mutex poisoned");
            let mut stmt = conn.prepare(
                "SELECT id, payload FROM queue ORDER BY priority ASC, id ASC LIMIT ?1",
            )?;
            let rows = stmt.query_map([limit as i64], |row| {
                let id: i64 = row.get(0)?;
                let payload: String = row.get(1)?;
                Ok((id, payload))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (id, payload) = row?;
                let event: ProcessedEvent = match serde_json::from_str(&payload) {
                    Ok(ev) => ev,
                    Err(_) => continue,
                };
                out.push(QueueItem { id, event });
            }
            Ok(out)
        })
        .await??;
        Ok(items)
    }

    /// Remove items by id after a successful HEC send.
    pub async fn ack(&self, ids: &[i64]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let ids = ids.to_vec();
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = conn.lock().expect("queue mutex poisoned");
            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare("DELETE FROM queue WHERE id = ?1")?;
                for id in &ids {
                    stmt.execute([*id])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
        .await??;
        Ok(())
    }

    /// Current row count.
    pub async fn depth(&self) -> Result<u64> {
        let conn = self.conn.clone();
        let depth = tokio::task::spawn_blocking(move || -> Result<u64> {
            let conn = conn.lock().expect("queue mutex poisoned");
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM queue", [], |r| r.get(0))
                .optional()?
                .unwrap_or(0);
            Ok(count as u64)
        })
        .await??;
        Ok(depth)
    }

    /// Wipe the entire queue. Called from the MCP `reset` tool.
    pub async fn clear(&self) -> Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().expect("queue mutex poisoned");
            conn.execute("DELETE FROM queue", [])?;
            Ok(())
        })
        .await??;
        Ok(())
    }
}

fn ensure_capacity(conn: &mut Connection, max_bytes: u64) -> Result<()> {
    // Rough size estimate via page_count * page_size. If we're over the cap,
    // drop the oldest LOW-priority rows; if none, drop the oldest MEDIUM; if
    // still none, drop the oldest HIGH (last resort).
    let page_count: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    let size = (page_count as u64).saturating_mul(page_size as u64);
    if size <= max_bytes {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for pri in [2_i64, 1, 0] {
        let deleted: usize = tx.execute(
            "DELETE FROM queue WHERE id IN (
                 SELECT id FROM queue WHERE priority = ?1 ORDER BY id ASC LIMIT 256
             )",
            [pri],
        )?;
        if deleted > 0 {
            break;
        }
    }
    tx.commit()?;
    Ok(())
}

fn unix_secs_f64() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::ProcessedEvent;

    fn first(sig: &str, line: &str) -> ProcessedEvent {
        ProcessedEvent::FirstOccurrence {
            signature: sig.into(),
            line: line.into(),
            ts: 1.0,
            source: "t".into(),
        }
    }

    fn collapsed(sig: &str, count: u64) -> ProcessedEvent {
        ProcessedEvent::Collapsed {
            signature: sig.into(),
            count,
            window_secs: 30.0,
            first_seen: 1.0,
            last_seen: 31.0,
            sample: "boom".into(),
            source: "t".into(),
            classification: None,
        }
    }

    #[tokio::test]
    async fn drains_high_priority_first() {
        let q = Queue::open(":memory:", 1 << 30).unwrap();
        // Enqueue 2 collapsed (medium) then 1 first-occurrence (high).
        q.enqueue(&collapsed("a", 10)).await.unwrap();
        q.enqueue(&collapsed("b", 20)).await.unwrap();
        q.enqueue(&first("c", "ERROR boom")).await.unwrap();

        let batch = q.peek_batch(10).await.unwrap();
        assert_eq!(batch.len(), 3);
        assert!(matches!(batch[0].event, ProcessedEvent::FirstOccurrence { .. }));
        assert!(matches!(batch[1].event, ProcessedEvent::Collapsed { .. }));
        assert!(matches!(batch[2].event, ProcessedEvent::Collapsed { .. }));

        q.ack(&[batch[0].id, batch[1].id]).await.unwrap();
        let depth = q.depth().await.unwrap();
        assert_eq!(depth, 1);
    }

    #[tokio::test]
    async fn ack_and_clear_work() {
        let q = Queue::open(":memory:", 1 << 30).unwrap();
        for i in 0..5 {
            q.enqueue(&collapsed("x", i)).await.unwrap();
        }
        assert_eq!(q.depth().await.unwrap(), 5);
        let batch = q.peek_batch(10).await.unwrap();
        let ids: Vec<i64> = batch.iter().map(|x| x.id).collect();
        q.ack(&ids).await.unwrap();
        assert_eq!(q.depth().await.unwrap(), 0);

        q.enqueue(&first("y", "boom")).await.unwrap();
        q.clear().await.unwrap();
        assert_eq!(q.depth().await.unwrap(), 0);
    }
}
