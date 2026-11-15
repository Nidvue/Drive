// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The telemetry and transfer surfaces, exercised through the public SDK.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use nidvue_core::config::AppConfig;
use nidvue_core::event::{Direction, Event};
use nidvue_core::metrics::MetricsSnapshot;
use nidvue_sdk::{Client, TransferState};

async fn client_with(dir: &std::path::Path, configure: impl FnOnce(&mut AppConfig)) -> Client {
    let mut config = AppConfig::default();
    configure(&mut config);
    config.save(dir).unwrap();
    Client::open(dir).await.unwrap()
}

fn scrape(url: &str) -> String {
    let addr: SocketAddr = url
        .trim_start_matches("http://")
        .trim_end_matches("/metrics")
        .parse()
        .unwrap();
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .write_all(b"GET /metrics HTTP/1.1\r\nHost: test\r\n\r\n")
        .unwrap();
    let mut out = String::new();
    stream.read_to_string(&mut out).unwrap();
    out
}

async fn wait_for(mut check: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if check() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    check()
}

#[tokio::test]
async fn the_scrape_endpoint_serves_live_values() {
    let dir = tempfile::tempdir().unwrap();
    let client = client_with(dir.path(), |c| {
        c.log.prometheus_bind = Some("127.0.0.1:0".into());
    })
    .await;
    let status = client.status().await.unwrap();
    let url = status.prometheus_url.expect("a bound scrape URL");
    assert!(url.ends_with("/metrics"), "{url}");
    assert!(wait_for(|| scrape(&url).contains("nidvue_index_entries")).await);
    let body = scrape(&url);
    assert!(
        body.contains("nidvue_cache_capacity_bytes 21474836480"),
        "{body}"
    );
    assert!(body.contains("nidvue_paused 0"), "{body}");
    assert!(body.contains("nidvue_health 1"), "{body}");
    assert!(body.contains("nidvue_logged_in 0"), "{body}");
}

#[tokio::test]
async fn the_scrape_agrees_with_the_status_view() {
    let dir = tempfile::tempdir().unwrap();
    let client = client_with(dir.path(), |c| {
        c.log.prometheus_bind = Some("127.0.0.1:0".into());
    })
    .await;
    client
        .index()
        .upsert_entry(&nidvue_cache::EntryRow {
            path: "/a".into(),
            kind: nidvue_core::types::EntryKind::File,
            size: 1,
            mtime_ms: 0,
            hash: None,
            remote_rev: None,
            dirty: false,
        })
        .unwrap();
    let status = client.status().await.unwrap();
    let url = status.prometheus_url.clone().expect("a scrape URL");
    assert!(
        wait_for(
            || scrape(&url).contains(&format!("nidvue_index_entries {}", status.index_entries))
        )
        .await,
        "the endpoint and the status view disagree"
    );
    let body = scrape(&url);
    assert!(
        body.contains(&format!("nidvue_cache_bytes {}", status.cache.bytes)),
        "{body}"
    );
    assert!(
        body.contains(&format!(
            "nidvue_cache_capacity_bytes {}",
            status.cache.capacity
        )),
        "{body}"
    );
}

#[tokio::test]
async fn the_metric_snapshot_tracks_pause_state() {
    let dir = tempfile::tempdir().unwrap();
    let client = client_with(dir.path(), |_| {}).await;
    let before = client.metrics_snapshot().await.unwrap();
    assert_eq!(before.get("nidvue_paused"), Some(0.0));
    client.set_paused(true).unwrap();
    let after = client.metrics_snapshot().await.unwrap();
    assert_eq!(after.get("nidvue_paused"), Some(1.0));
    assert_eq!(after.service, "nidvue-drive");
    assert_eq!(after.version, nidvue_core::VERSION);
    assert!(after.samples.len() >= 13);
}

#[tokio::test]
async fn transfers_are_folded_from_the_event_bus() {
    let dir = tempfile::tempdir().unwrap();
    let client = client_with(dir.path(), |_| {}).await;
    assert!(client.transfers().is_empty());

    client.publish(Event::TransferQueued {
        path: "/Archive/a.bin".into(),
        size: 4096,
        direction: Direction::Upload,
    });
    assert!(
        wait_for(|| client.transfers().len() == 1).await,
        "queued transfer never appeared"
    );
    assert_eq!(client.transfers()[0].state, TransferState::Queued);
    assert_eq!(client.stats().await.unwrap().queue_depth, 1);

    client.publish(Event::TransferProgress {
        path: "/Archive/a.bin".into(),
        direction: Direction::Upload,
        done: 512,
        total: 1024,
    });
    assert!(
        wait_for(|| client.transfers()[0].state == TransferState::Running).await,
        "progress never reached the list"
    );
    assert_eq!(client.transfers()[0].fraction(), 0.5);
    let stats = client.stats().await.unwrap();
    assert_eq!(stats.active_transfers, 1);
    assert_eq!(stats.queue_depth, 0);

    client.publish(Event::TransferDone {
        path: "/Archive/a.bin".into(),
        direction: Direction::Upload,
        hash: "abc".into(),
    });
    assert!(
        wait_for(|| client.transfers()[0].state == TransferState::Done).await,
        "completion never reached the list"
    );

    let status = client.status().await.unwrap();
    assert_eq!(status.transfers.len(), 1);
    assert_eq!(status.transfers[0].path, "/Archive/a.bin");

    client.clear_transfers();
    assert!(client.transfers().is_empty());
}

#[tokio::test]
async fn status_reports_the_secret_backend_and_the_transports() {
    let dir = tempfile::tempdir().unwrap();
    let client = client_with(dir.path(), |c| {
        c.log.otel_endpoint = Some("http://127.0.0.1:9".into());
        c.secret.backend = nidvue_core::config::SecretBackend::File;
    })
    .await;
    let status = client.status().await.unwrap();
    assert_eq!(status.secret_backend, "file");
    assert_eq!(status.otlp_endpoint.as_deref(), Some("http://127.0.0.1:9"));
    assert!(status.prometheus_url.is_none());

    let json = serde_json::to_value(&status).unwrap();
    for key in [
        "transfers",
        "secret_backend",
        "prometheus_url",
        "otlp_endpoint",
    ] {
        assert!(json.get(key).is_some(), "status is missing {key}");
    }
}

#[tokio::test]
async fn the_schema_documents_the_new_fields() {
    let schema: serde_json::Value = serde_json::from_str(&Client::json_schema()).unwrap();
    let text = serde_json::to_string(&schema).unwrap();
    assert!(text.contains("transfers"));
    assert!(text.contains("secret_backend"));
}

#[tokio::test]
async fn the_snapshot_renders_both_wire_formats() {
    let dir = tempfile::tempdir().unwrap();
    let client = client_with(dir.path(), |_| {}).await;
    let snapshot: MetricsSnapshot = client.metrics_snapshot().await.unwrap();
    assert!(snapshot
        .render_prometheus()
        .contains("nidvue_index_entries"));
    let otlp = snapshot.to_otlp_json(1);
    assert_eq!(
        otlp["resourceMetrics"][0]["scopeMetrics"][0]["metrics"][0]["name"],
        "nidvue_cache_bytes"
    );
}
