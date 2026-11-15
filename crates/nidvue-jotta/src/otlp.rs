// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Opt-in OpenTelemetry export.
//!
//! `log.otel_endpoint` turns this on. One span is exported per finished sync
//! pass, plus an error span per failed transfer, as OTLP/HTTP JSON to
//! `<endpoint>/v1/traces`. Batching is per event: a sync pass is not a
//! high-frequency event, and a failed transfer is rare by definition.

use nidvue_core::error::{NidvueError, Result};
use nidvue_core::event::{Event, EventEnvelope, HealthState};
use nidvue_core::hash::Hash;
use nidvue_core::time::now_ms;

/// One span, before it becomes OTLP JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanRecord {
    /// Span name, e.g. `sync pass`.
    pub name: String,
    /// Start, ms since the epoch.
    pub start_ms: i64,
    /// End, ms since the epoch.
    pub end_ms: i64,
    /// Key/value attributes.
    pub attributes: Vec<(String, String)>,
    /// True when the operation failed.
    pub failed: bool,
}

/// Turn one engine event into the spans it should export.
///
/// Only events that describe a finished unit of work produce a span; progress
/// events would turn the exporter into a busy loop.
pub fn spans_from_event(envelope: &EventEnvelope) -> Vec<SpanRecord> {
    let ts = envelope.ts_ms;
    match &envelope.event {
        Event::SyncFinished {
            pair,
            scanned,
            planned,
            applied,
            conflicts,
        } => vec![SpanRecord {
            name: "sync pass".into(),
            start_ms: ts,
            end_ms: ts,
            attributes: vec![
                ("pair".into(), pair.clone()),
                ("scanned".into(), scanned.to_string()),
                ("planned".into(), planned.to_string()),
                ("applied".into(), applied.to_string()),
                ("conflicts".into(), conflicts.to_string()),
            ],
            failed: false,
        }],
        Event::TransferFailed {
            path,
            direction,
            error,
            retryable,
        } => vec![SpanRecord {
            name: "transfer".into(),
            start_ms: ts,
            end_ms: ts,
            attributes: vec![
                ("path".into(), path.clone()),
                ("direction".into(), format!("{direction:?}").to_lowercase()),
                ("error".into(), error.clone()),
                ("retryable".into(), retryable.to_string()),
            ],
            failed: true,
        }],
        Event::Health {
            state: HealthState::Error,
            detail,
        } => vec![SpanRecord {
            name: "health".into(),
            start_ms: ts,
            end_ms: ts,
            attributes: vec![("detail".into(), detail.clone().unwrap_or_default())],
            failed: true,
        }],
        _ => Vec::new(),
    }
}

/// An OTLP/HTTP JSON exporter for `<endpoint>/v1/traces`.
#[derive(Debug, Clone)]
pub struct OtlpExporter {
    http: reqwest::Client,
    url: String,
    service: String,
}

impl OtlpExporter {
    /// Build an exporter. A bare endpoint gains the `/v1/traces` suffix.
    pub fn new(endpoint: &str) -> Result<Self> {
        let trimmed = endpoint.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(NidvueError::Config("otel_endpoint is empty".into()));
        }
        let url = if trimmed.ends_with("/v1/traces") {
            trimmed.to_string()
        } else {
            format!("{trimmed}/v1/traces")
        };
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| NidvueError::Network(format!("otlp client: {e}")))?;
        Ok(Self {
            http,
            url,
            service: "nidvue-drive".into(),
        })
    }

    /// The target URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The OTLP JSON body for a batch of spans.
    pub fn body(&self, spans: &[SpanRecord]) -> serde_json::Value {
        let spans: Vec<serde_json::Value> = spans.iter().map(span_json).collect();
        serde_json::json!({
            "resourceSpans": [{
                "resource": {
                    "attributes": [{
                        "key": "service.name",
                        "value": { "stringValue": self.service },
                    }],
                },
                "scopeSpans": [{
                    "scope": { "name": "nidvue" },
                    "spans": spans,
                }],
            }],
        })
    }

    /// POST the spans. A non-2xx answer is an error, never a silent drop.
    pub async fn send(&self, spans: &[SpanRecord]) -> Result<()> {
        if spans.is_empty() {
            return Ok(());
        }
        let response = self
            .http
            .post(&self.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&self.body(spans))
            .send()
            .await
            .map_err(|e| NidvueError::Network(format!("otlp export: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(NidvueError::Server {
                status: status.as_u16(),
                message: format!(
                    "otlp export rejected: {}",
                    response.text().await.unwrap_or_default()
                ),
            });
        }
        Ok(())
    }
}

fn span_json(span: &SpanRecord) -> serde_json::Value {
    let trace = Hash::of(format!("{}:{}", span.name, span.start_ms).as_bytes());
    let trace_hex = trace.as_str().chars().take(32).collect::<String>();
    let span_hex = trace.as_str().chars().skip(32).take(16).collect::<String>();
    let attributes: Vec<serde_json::Value> = span
        .attributes
        .iter()
        .map(|(key, value)| {
            serde_json::json!({
                "key": key,
                "value": { "stringValue": value },
            })
        })
        .collect();
    serde_json::json!({
        "traceId": trace_hex,
        "spanId": span_hex,
        "name": span.name,
        "kind": 1,
        "startTimeUnixNano": (span.start_ms.max(0) as u64 * 1_000_000).to_string(),
        "endTimeUnixNano": (span.end_ms.max(0) as u64 * 1_000_000).to_string(),
        "attributes": attributes,
        "status": { "code": if span.failed { 2 } else { 1 } },
    })
}

/// A span for "now", used when a caller has no event to convert.
pub fn now_span(name: &str) -> SpanRecord {
    let ts = now_ms();
    SpanRecord {
        name: name.into(),
        start_ms: ts,
        end_ms: ts,
        attributes: Vec::new(),
        failed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nidvue_core::event::Direction;

    #[test]
    fn a_finished_pass_becomes_one_span() {
        let envelope = EventEnvelope::new(Event::SyncFinished {
            pair: "docs".into(),
            scanned: 10,
            planned: 3,
            applied: 2,
            conflicts: 1,
        });
        let spans = spans_from_event(&envelope);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].name, "sync pass");
        assert!(!spans[0].failed);
        assert!(spans[0]
            .attributes
            .contains(&("pair".to_string(), "docs".to_string())));
    }

    #[test]
    fn progress_events_produce_no_spans() {
        let envelope = EventEnvelope::new(Event::TransferProgress {
            path: "/a".into(),
            direction: Direction::Upload,
            done: 1,
            total: 2,
        });
        assert!(spans_from_event(&envelope).is_empty());
    }

    #[test]
    fn a_failed_transfer_becomes_an_error_span() {
        let envelope = EventEnvelope::new(Event::TransferFailed {
            path: "/a".into(),
            direction: Direction::Download,
            error: "boom".into(),
            retryable: true,
        });
        let spans = spans_from_event(&envelope);
        assert_eq!(spans.len(), 1);
        assert!(spans[0].failed);
        let body = OtlpExporter::new("http://127.0.0.1:4318")
            .unwrap()
            .body(&spans);
        assert_eq!(
            body["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["status"]["code"],
            2
        );
    }

    #[test]
    fn the_endpoint_gains_the_traces_suffix_once() {
        let a = OtlpExporter::new("http://collector:4318").unwrap();
        assert_eq!(a.url(), "http://collector:4318/v1/traces");
        let b = OtlpExporter::new("http://collector:4318/").unwrap();
        assert_eq!(b.url(), "http://collector:4318/v1/traces");
        let c = OtlpExporter::new("http://collector:4318/v1/traces").unwrap();
        assert_eq!(c.url(), "http://collector:4318/v1/traces");
        assert!(OtlpExporter::new("  ").is_err());
    }

    #[tokio::test]
    async fn spans_are_posted_as_otlp_json() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .await
                .unwrap();
            request
        });

        let exporter = OtlpExporter::new(&format!("http://{addr}")).unwrap();
        let span = now_span("sync pass");
        exporter.send(std::slice::from_ref(&span)).await.unwrap();
        let request = server.await.unwrap();
        assert!(request.starts_with("POST /v1/traces "), "{request}");
        assert!(request.contains("application/json"), "{request}");
        assert!(request.contains("\"resourceSpans\""), "{request}");
        assert!(request.contains("service.name"), "{request}");
        assert!(request.contains("\"name\":\"sync pass\""), "{request}");
    }

    #[tokio::test]
    async fn a_rejected_export_is_an_error() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 1024];
            let _ = stream.read(&mut buf).await;
            stream
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 3\r\nConnection: close\r\n\r\nno",
                )
                .await
                .unwrap();
        });

        let exporter = OtlpExporter::new(&format!("http://{addr}")).unwrap();
        let err = exporter.send(&[now_span("sync pass")]).await.unwrap_err();
        assert!(err.to_string().contains("400"), "{err}");
    }
}
