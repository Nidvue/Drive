// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The headless control surface, exercised as a user runs it.

use std::path::Path;
use std::process::{Command, Output};

fn run(state: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nidvue-cli"))
        .arg("--state")
        .arg(state)
        .args(args)
        .env_remove("NIDVUE_LOG")
        .output()
        .expect("the CLI runs")
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn a_pair_is_added_listed_filtered_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();

    stdout(&run(
        state,
        &[
            "sync",
            "add",
            "docs",
            "/tmp/docs",
            "/Archive/Docs",
            "--exclude",
            "**/node_modules",
            "--exclude",
            "*.tmp",
            "--conflict-policy",
            "keep_local",
        ],
    ));

    let listed = stdout(&run(state, &["--json", "sync", "list"]));
    let pairs: serde_json::Value = serde_json::from_str(&listed).unwrap();
    assert_eq!(pairs[0]["name"], "docs");
    assert_eq!(
        pairs[0]["exclude"],
        serde_json::json!(["**/node_modules", "*.tmp"])
    );
    assert_eq!(pairs[0]["conflict_policy"], "prefer_local");

    stdout(&run(state, &["sync", "pause", "docs"]));
    let listed = stdout(&run(state, &["--json", "sync", "list"]));
    let pairs: serde_json::Value = serde_json::from_str(&listed).unwrap();
    assert_eq!(pairs[0]["paused"], true);

    stdout(&run(state, &["sync", "remove", "docs"]));
    let listed = stdout(&run(state, &["--json", "sync", "list"]));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&listed).unwrap(),
        serde_json::json!([])
    );
}

#[test]
fn config_get_set_reach_a_nested_value_and_reject_a_bad_one() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();
    stdout(&run(
        state,
        &["sync", "add", "docs", "/tmp/docs", "/Archive/Docs"],
    ));

    assert_eq!(
        stdout(&run(state, &["config", "get", "sync.poll_interval_ms"])).trim(),
        "5000"
    );
    stdout(&run(
        state,
        &["config", "set", "sync.poll_interval_ms", "15000"],
    ));
    assert_eq!(
        stdout(&run(state, &["config", "get", "sync.poll_interval_ms"])).trim(),
        "15000"
    );

    stdout(&run(
        state,
        &["config", "set", "sync.pairs.docs.exclude", r#"["**/.git"]"#],
    ));
    assert_eq!(
        stdout(&run(
            state,
            &["--json", "config", "get", "sync.pairs.docs.exclude"]
        ))
        .trim(),
        r#"[
  "**/.git"
]"#
    );

    let bad = run(state, &["config", "set", "sync.nope", "1"]);
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("unknown configuration key"));

    let zero = run(state, &["config", "set", "cache.max_size_bytes", "0"]);
    assert!(!zero.status.success());
    assert_eq!(
        stdout(&run(state, &["config", "get", "cache.max_size_bytes"])).trim(),
        "21474836480",
        "a refused value leaves the configuration alone"
    );
}

#[test]
fn config_list_names_every_key_it_can_set() {
    let dir = tempfile::tempdir().unwrap();
    let listed = stdout(&run(dir.path(), &["config", "list"]));
    for key in [
        "cache.max_size_bytes",
        "sync.poll_interval_ms",
        "network.max_concurrency",
        "mount.read_only",
        "ui.theme",
    ] {
        assert!(listed.contains(key), "{key} missing from config list");
    }
    let listed = stdout(&run(dir.path(), &["--json", "config", "list"]));
    let config: serde_json::Value = serde_json::from_str(&listed).unwrap();
    assert!(config["cache.max_size_bytes"].is_number());
}
