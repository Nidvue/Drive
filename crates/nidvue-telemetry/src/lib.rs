// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The two metric transports: a Prometheus scrape endpoint and an OTLP/HTTP
//! exporter.
//!
//! Both read the same [`MetricsCache`], so a scrape and an export can never
//! disagree about a value. Rendering is `nidvue-core::metrics`; this crate only
//! moves bytes.
//!
//! ```
//! use nidvue_core::metrics::{MetricSample, MetricsSnapshot};
//! use nidvue_telemetry::MetricsCache;
//!
//! let cache = MetricsCache::new();
//! cache.store(MetricsSnapshot {
//!     service: "nidvue-drive".into(),
//!     version: "0.2.0".into(),
//!     account: None,
//!     samples: vec![MetricSample::new("nidvue_index_entries", "Entries.", 3.0)],
//! });
//! assert!(cache.snapshot().render_prometheus().contains("nidvue_index_entries 3"));
//! ```

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use nidvue_core::error::{NidvueError, Result};
use nidvue_core::metrics::MetricsSnapshot;

/// The snapshot both transports read.
#[derive(Clone, Default)]
pub struct MetricsCache {
    inner: Arc<RwLock<MetricsSnapshot>>,
}

impl MetricsCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the snapshot.
    pub fn store(&self, snapshot: MetricsSnapshot) {
        if let Ok(mut guard) = self.inner.write() {
            *guard = snapshot;
        }
    }

    /// Read the snapshot; an empty one is returned only if the lock is poisoned.
    pub fn snapshot(&self) -> MetricsSnapshot {
        self.inner
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

/// A running scrape endpoint.
pub struct PrometheusServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl PrometheusServer {
    /// Bind `bind` (host:port) and serve `GET /metrics` until dropped.
    ///
    /// Port 0 asks the kernel for a free port; read it back with [`Self::addr`].
    pub fn start(bind: &str, cache: MetricsCache) -> Result<Self> {
        let listener = TcpListener::bind(bind)
            .map_err(|e| NidvueError::Config(format!("prometheus bind {bind}: {e}")))?;
        let addr = listener
            .local_addr()
            .map_err(|e| NidvueError::Config(format!("prometheus bind {bind}: {e}")))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| NidvueError::Config(format!("prometheus bind {bind}: {e}")))?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("nidvue-prometheus".into())
            .spawn(move || accept_loop(listener, cache, flag))
            .map_err(|e| NidvueError::Config(format!("prometheus thread: {e}")))?;
        tracing::info!(%addr, "prometheus endpoint listening on /metrics");
        Ok(Self {
            addr,
            stop,
            thread: Some(thread),
        })
    }

    /// The bound address, with the real port when 0 was requested.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The scrape URL a collector should use.
    pub fn url(&self) -> String {
        format!("http://{}/metrics", self.addr)
    }
}

impl Drop for PrometheusServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn accept_loop(listener: TcpListener, cache: MetricsCache, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _peer)) => {
                let _ = handle_connection(stream, &cache);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                tracing::warn!(error = %e, "prometheus accept failed");
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, cache: &MetricsCache) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let path = path.split('?').next().unwrap_or(path);
    let response = match path {
        "/metrics" => {
            let body = cache.snapshot().render_prometheus();
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
        }
        "/healthz" => "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\
                       Content-Length: 3\r\nConnection: close\r\n\r\nok\n"
            .to_string(),
        _ => "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\
              Content-Length: 10\r\nConnection: close\r\n\r\nnot found\n"
            .to_string(),
    };
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

/// Posts OTLP/HTTP JSON to `<endpoint>/v1/metrics`.
#[derive(Clone)]
pub struct OtlpExporter {
    endpoint: String,
    client: reqwest::Client,
    cache: MetricsCache,
}

impl OtlpExporter {
    /// An exporter for a collector base URL, e.g. `http://127.0.0.1:4318`.
    ///
    /// A trailing slash and a trailing `/v1/metrics` are both accepted.
    pub fn new(endpoint: &str, cache: MetricsCache) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(format!("nidvue-drive/{}", nidvue_core::VERSION))
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| NidvueError::Network(format!("otlp client: {e}")))?;
        Ok(Self {
            endpoint: metrics_url(endpoint),
            client,
            cache,
        })
    }

    /// The URL exports are posted to.
    pub fn url(&self) -> &str {
        &self.endpoint
    }

    /// Export the current snapshot once.
    ///
    /// A collector that is down is a logged warning, never an error the engine
    /// acts on: telemetry must not be able to stop a sync.
    pub async fn export_once(&self) -> Result<()> {
        let snapshot = self.cache.snapshot();
        if snapshot.samples.is_empty() {
            return Ok(());
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let body = snapshot.to_otlp_json(now);
        let response = self
            .client
            .post(&self.endpoint)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NidvueError::Network(format!("otlp export: {e}")))?;
        if !response.status().is_success() {
            return Err(NidvueError::Network(format!(
                "otlp export: collector answered {}",
                response.status()
            )));
        }
        Ok(())
    }

    /// Export on an interval until the returned task is aborted.
    pub fn spawn(self, every: Duration) -> tokio::task::JoinHandle<()> {
        let url = self.endpoint.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(every);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if let Err(e) = self.export_once().await {
                    tracing::warn!(%url, error = %e, "otlp export failed");
                }
            }
        })
    }
}

fn metrics_url(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    if trimmed.ends_with("/v1/metrics") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1/metrics")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nidvue_core::metrics::MetricSample;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn cache() -> MetricsCache {
        let cache = MetricsCache::new();
        cache.store(MetricsSnapshot {
            service: "nidvue-drive".into(),
            version: "0.2.0".into(),
            account: Some("alice".into()),
            samples: vec![MetricSample::new(
                "nidvue_index_entries",
                "Entries in the metadata index.",
                42.0,
            )],
        });
        cache
    }

    fn get(addr: SocketAddr, path: &str) -> String {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .write_all(format!("GET {path} HTTP/1.1\r\nHost: test\r\n\r\n").as_bytes())
            .unwrap();
        let mut out = String::new();
        stream.read_to_string(&mut out).unwrap();
        out
    }

    #[test]
    fn scrape_serves_the_snapshot() {
        let server = PrometheusServer::start("127.0.0.1:0", cache()).unwrap();
        assert_ne!(server.addr().port(), 0);
        assert!(server.url().ends_with("/metrics"));
        let response = get(server.addr(), "/metrics");
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(response.contains("text/plain; version=0.0.4"));
        assert!(response.contains("nidvue_index_entries 42"));
        assert!(response.contains("nidvue_build_info{version=\"0.2.0\"} 1"));
    }

    #[test]
    fn scrape_reflects_a_later_snapshot() {
        let cache = cache();
        let server = PrometheusServer::start("127.0.0.1:0", cache.clone()).unwrap();
        cache.store(MetricsSnapshot {
            service: "nidvue-drive".into(),
            version: "0.2.0".into(),
            account: None,
            samples: vec![MetricSample::new("nidvue_index_entries", "Entries.", 7.0)],
        });
        let response = get(server.addr(), "/metrics");
        assert!(response.contains("nidvue_index_entries 7"), "{response}");
    }

    #[test]
    fn query_string_and_health_are_handled() {
        let server = PrometheusServer::start("127.0.0.1:0", cache()).unwrap();
        assert!(get(server.addr(), "/metrics?x=1").contains("nidvue_index_entries 42"));
        assert!(get(server.addr(), "/healthz").contains("200 OK"));
        let missing = get(server.addr(), "/nope");
        assert!(missing.contains("404 Not Found"), "{missing}");
    }

    #[test]
    fn dropping_the_server_frees_the_port() {
        let addr = {
            let server = PrometheusServer::start("127.0.0.1:0", cache()).unwrap();
            server.addr()
        };
        let rebound = TcpListener::bind(addr);
        assert!(rebound.is_ok(), "port {addr} was not released");
    }

    #[test]
    fn exporter_normalises_the_endpoint() {
        let cache = MetricsCache::new();
        for input in [
            "http://127.0.0.1:4318",
            "http://127.0.0.1:4318/",
            "http://127.0.0.1:4318/v1/metrics",
        ] {
            let exporter = OtlpExporter::new(input, cache.clone()).unwrap();
            assert_eq!(exporter.url(), "http://127.0.0.1:4318/v1/metrics");
        }
    }

    #[tokio::test]
    async fn exporter_posts_otlp_json() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/metrics"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let exporter = OtlpExporter::new(&server.uri(), cache()).unwrap();
        exporter.export_once().await.unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        let metric = &body["resourceMetrics"][0]["scopeMetrics"][0]["metrics"][0];
        assert_eq!(metric["name"], "nidvue_index_entries");
        assert_eq!(metric["gauge"]["dataPoints"][0]["asDouble"], 42.0);
        assert_eq!(
            body["resourceMetrics"][0]["resource"]["attributes"][0]["value"]["stringValue"],
            "nidvue-drive"
        );
    }

    #[tokio::test]
    async fn a_dead_collector_is_an_error_not_a_panic() {
        let exporter = OtlpExporter::new("http://127.0.0.1:1", cache()).unwrap();
        let err = exporter.export_once().await.unwrap_err();
        assert!(matches!(err, NidvueError::Network(_)), "{err}");
    }

    #[tokio::test]
    async fn a_non_success_status_is_reported() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let exporter = OtlpExporter::new(&server.uri(), cache()).unwrap();
        let err = exporter.export_once().await.unwrap_err();
        assert!(err.to_string().contains("503"), "{err}");
    }

    #[tokio::test]
    async fn an_empty_snapshot_is_not_exported() {
        let server = MockServer::start().await;
        let exporter = OtlpExporter::new(&server.uri(), MetricsCache::new()).unwrap();
        exporter.export_once().await.unwrap();
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}
