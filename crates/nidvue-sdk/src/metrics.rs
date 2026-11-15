// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The opt-in Prometheus endpoint.
//!
//! `log.prometheus_bind` turns this on; the scrape answers from the same
//! snapshot `status --json` prints, so a dashboard and the UI never disagree.

use std::net::SocketAddr;

use nidvue_core::error::{NidvueError, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::Client;

/// A running scrape endpoint. Dropping it stops the listener.
pub struct MetricsHandle {
    addr: SocketAddr,
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

impl MetricsHandle {
    /// The bound address, useful when the caller asked for port 0.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Drop for MetricsHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.task.abort();
    }
}

impl Client {
    /// Serve `/metrics` on `bind` until the returned handle is dropped.
    pub async fn start_metrics(&self, bind: &str) -> Result<MetricsHandle> {
        let addr: SocketAddr = bind
            .parse()
            .map_err(|e| NidvueError::Config(format!("prometheus_bind {bind:?}: {e}")))?;
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| NidvueError::Network(format!("prometheus bind {bind}: {e}")))?;
        let addr = listener
            .local_addr()
            .map_err(|e| NidvueError::Network(format!("prometheus local_addr: {e}")))?;
        let client = self.clone();
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let task = tokio::spawn(async move {
            loop {
                let accepted = tokio::select! {
                    _ = token.cancelled() => break,
                    accepted = listener.accept() => accepted,
                };
                match accepted {
                    Ok((stream, _)) => {
                        let client = client.clone();
                        tokio::spawn(async move {
                            let _ = answer(client, stream).await;
                        });
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(MetricsHandle { addr, cancel, task })
    }

    /// The scrape body, exposed so a caller can log or test it directly.
    pub async fn metrics_text(&self) -> Result<String> {
        let status = self.status().await?;
        let paused = u8::from(status.paused);
        let logged_in = u8::from(status.logged_in);
        let mut out = String::with_capacity(1024);
        out.push_str("# HELP nidvue_cache_bytes Bytes in the blob cache.\n");
        out.push_str("# TYPE nidvue_cache_bytes gauge\n");
        out.push_str(&format!("nidvue_cache_bytes {}\n", status.cache.bytes));
        out.push_str("# HELP nidvue_cache_capacity Configured cache cap in bytes.\n");
        out.push_str("# TYPE nidvue_cache_capacity gauge\n");
        out.push_str(&format!(
            "nidvue_cache_capacity {}\n",
            status.cache_capacity
        ));
        out.push_str("# HELP nidvue_index_entries Entries in the metadata index.\n");
        out.push_str("# TYPE nidvue_index_entries gauge\n");
        out.push_str(&format!("nidvue_index_entries {}\n", status.index_entries));
        out.push_str("# HELP nidvue_conflicts_open Conflicts awaiting a decision.\n");
        out.push_str("# TYPE nidvue_conflicts_open gauge\n");
        out.push_str(&format!(
            "nidvue_conflicts_open {}\n",
            status.conflicts_open
        ));
        out.push_str("# HELP nidvue_paused 1 when sync is paused.\n");
        out.push_str("# TYPE nidvue_paused gauge\n");
        out.push_str(&format!("nidvue_paused {paused}\n"));
        out.push_str("# HELP nidvue_logged_in 1 when a token is present.\n");
        out.push_str("# TYPE nidvue_logged_in gauge\n");
        out.push_str(&format!("nidvue_logged_in {logged_in}\n"));
        out.push_str("# HELP nidvue_bandwidth_limit_bps Aggregate cap, 0 = unlimited.\n");
        out.push_str("# TYPE nidvue_bandwidth_limit_bps gauge\n");
        out.push_str(&format!(
            "nidvue_bandwidth_limit_bps {}\n",
            status.bandwidth_limit_bps
        ));
        out.push_str("# HELP nidvue_uptime_seconds Seconds since the client opened.\n");
        out.push_str("# TYPE nidvue_uptime_seconds gauge\n");
        out.push_str(&format!("nidvue_uptime_seconds {}\n", status.uptime_s));
        Ok(out)
    }
}

async fn answer(client: Client, mut stream: TcpStream) -> Result<()> {
    let mut buf = [0u8; 1024];
    let read = stream
        .read(&mut buf)
        .await
        .map_err(|e| NidvueError::Network(format!("metrics read: {e}")))?;
    let request = String::from_utf8_lossy(&buf[..read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let (status, body) = if path == "/metrics" {
        match client.metrics_text().await {
            Ok(body) => ("200 OK", body),
            Err(e) => ("500 Internal Server Error", format!("# error: {e}\n")),
        }
    } else {
        ("404 Not Found", String::from("not found\n"))
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|e| NidvueError::Network(format!("metrics write: {e}")))?;
    let _ = stream.shutdown().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_scrape_lists_the_gauges_and_serves_them_over_http() {
        let dir = tempfile::tempdir().unwrap();
        let client = Client::open(dir.path()).await.unwrap();
        let text = client.metrics_text().await.unwrap();
        assert!(text.contains("# TYPE nidvue_index_entries gauge"), "{text}");
        assert!(text.contains("nidvue_index_entries 0"), "{text}");
        assert!(text.contains("nidvue_paused 0"), "{text}");

        let handle = client.start_metrics("127.0.0.1:0").await.unwrap();
        let addr = handle.local_addr();
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut body = String::new();
        stream.read_to_string(&mut body).await.unwrap();
        assert!(body.starts_with("HTTP/1.1 200 OK"), "{body}");
        assert!(body.contains("nidvue_cache_bytes 0"), "{body}");

        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /nope HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut body = String::new();
        stream.read_to_string(&mut body).await.unwrap();
        assert!(body.starts_with("HTTP/1.1 404"), "{body}");
    }
}
