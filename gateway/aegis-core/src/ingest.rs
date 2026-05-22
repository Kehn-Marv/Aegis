//! Ingress for raw logs. TCP and UDP listeners feed lines into an mpsc
//! channel the dedup engine consumes.

use crate::event::IngestLine;
use anyhow::Context;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Listen for line-delimited logs over TCP and forward each line to `tx`.
pub async fn run_tcp(addr: SocketAddr, tx: mpsc::Sender<IngestLine>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind tcp listener at {addr}"))?;
    info!(%addr, "tcp ingest listening");

    loop {
        let (stream, peer) = listener.accept().await?;
        let tx = tx.clone();
        tokio::spawn(async move {
            debug!(%peer, "tcp connection opened");
            let reader = BufReader::new(stream);
            let mut lines = reader.lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let ev = IngestLine {
                            source: format!("tcp://{peer}"),
                            text: line,
                            ts_unix: unix_secs_f64(),
                        };
                        if tx.send(ev).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        warn!(%peer, error = %e, "tcp read failed");
                        break;
                    }
                }
            }
            debug!(%peer, "tcp connection closed");
        });
    }
}

/// Listen for line-delimited logs over UDP. Each datagram can carry one or
/// more `\n`-separated lines.
pub async fn run_udp(addr: SocketAddr, tx: mpsc::Sender<IngestLine>) -> anyhow::Result<()> {
    let socket = UdpSocket::bind(addr)
        .await
        .with_context(|| format!("bind udp listener at {addr}"))?;
    info!(%addr, "udp ingest listening");

    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let (n, peer) = socket.recv_from(&mut buf).await?;
        let Ok(s) = std::str::from_utf8(&buf[..n]) else {
            continue;
        };
        for line in s.lines() {
            if line.is_empty() {
                continue;
            }
            let ev = IngestLine {
                source: format!("udp://{peer}"),
                text: line.to_string(),
                ts_unix: unix_secs_f64(),
            };
            if tx.send(ev).await.is_err() {
                return Ok(());
            }
        }
    }
}

fn unix_secs_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
