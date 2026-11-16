// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! A value written by the CLI is the value the engine loads and acts on.
//!
//! The unit tests cover the dot-path walk; this covers the seam that actually
//! breaks — `config set` writing a file the client does not read back.

use std::path::Path;
use std::process::Command;

use nidvue_core::config::{AppConfig, SecretBackend};
use nidvue_sdk::Client;

fn cli(state: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_nidvue-cli"))
        .arg("--state")
        .arg(state)
        .args(args)
        .output()
        .expect("run nidvue-cli");
    assert!(
        output.status.success(),
        "nidvue-cli {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[tokio::test]
async fn a_value_set_by_the_cli_is_the_value_the_engine_loads() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();

    cli(state, &["config", "set", "cache.max_size_bytes", "1048576"]);
    cli(
        state,
        &["config", "set", "log.prometheus_bind", "127.0.0.1:0"],
    );
    cli(state, &["config", "set", "secret.backend", "file"]);
    cli(state, &["config", "set", "sync.workers", "2"]);

    let on_disk = AppConfig::load_or_default(state).unwrap();
    assert_eq!(on_disk.cache.max_size_bytes, 1_048_576);
    assert_eq!(on_disk.log.prometheus_bind.as_deref(), Some("127.0.0.1:0"));
    assert_eq!(on_disk.secret.backend, SecretBackend::File);
    assert_eq!(on_disk.sync.workers, 2);

    let client = Client::open(state).await.unwrap();
    assert_eq!(client.config().cache.max_size_bytes, 1_048_576);
    assert_eq!(client.config().sync.workers, 2);

    let status = client.status().await.unwrap();
    assert_eq!(status.cache.capacity, 1_048_576);
    assert_eq!(status.secret_backend, "file");
    assert!(
        status.prometheus_url.is_some(),
        "the engine did not start the endpoint the CLI configured"
    );
}

#[test]
fn config_get_reports_what_config_set_wrote() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();
    for (key, value) in [
        ("cache.readahead_bytes", "2097152"),
        ("log.level", "debug"),
        ("ui.theme", "dark"),
        ("sync.paused", "true"),
        ("network.max_concurrency", "3"),
    ] {
        cli(state, &["config", "set", key, value]);
        assert_eq!(cli(state, &["config", "get", key]), value, "{key}");
    }
}

#[test]
fn config_path_is_where_the_file_lands() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();
    cli(state, &["config", "set", "log.level", "warn"]);
    let reported = cli(state, &["config", "path"]);
    assert_eq!(reported, AppConfig::path_in(state).display().to_string());
    assert!(Path::new(&reported).exists(), "{reported} was not written");
    let text = std::fs::read_to_string(&reported).unwrap();
    assert!(text.contains("warn"), "{text}");
}

#[test]
fn an_invalid_value_is_refused_and_the_file_is_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();
    cli(state, &["config", "set", "cache.max_size_bytes", "4096"]);
    let before = std::fs::read_to_string(AppConfig::path_in(state)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_nidvue-cli"))
        .arg("--state")
        .arg(state)
        .args(["config", "set", "cache.max_size_bytes", "0"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "a zero cache was accepted");

    let after = std::fs::read_to_string(AppConfig::path_in(state)).unwrap();
    assert_eq!(before, after, "a refused value still rewrote the file");
}

#[test]
fn config_list_covers_every_key() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();
    let listed = cli(state, &["--json", "config", "list"]);
    let parsed: serde_json::Value = serde_json::from_str(&listed).unwrap();
    let keys = parsed.as_object().expect("an object of leaves");
    let expected =
        nidvue_cli::config_dot::keys(&AppConfig::load_or_default(state).unwrap()).unwrap();
    assert_eq!(keys.len(), expected.len());
    for key in expected {
        assert!(keys.contains_key(&key), "config list is missing {key}");
    }
}
