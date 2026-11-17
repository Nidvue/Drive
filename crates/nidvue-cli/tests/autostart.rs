// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! `nidvue-cli autostart`: the login entry a desktop reads, written and removed
//! through the CLI. `XDG_CONFIG_HOME` points at the test's own directory so the
//! machine running the suite is never touched.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(config_home: &Path, state: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nidvue-cli"))
        .arg("--state")
        .arg(state)
        .args(args)
        .env("XDG_CONFIG_HOME", config_home)
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

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_str(stdout(output).trim()).expect("the CLI prints one JSON document")
}

fn entry_path(config_home: &Path) -> PathBuf {
    config_home.join("autostart").join("nidvue-drive.desktop")
}

#[test]
fn the_login_entry_is_written_read_back_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    let config_home = dir.path().join("config");
    let state = dir.path().join("state");
    std::fs::create_dir_all(&config_home).unwrap();

    let status = json(&run(&config_home, &state, &["--json", "autostart", "status"]));
    assert_eq!(status["enabled"], false);
    assert_eq!(status["exec"], serde_json::Value::Null);
    assert!(!entry_path(&config_home).exists());

    stdout(&run(
        &config_home,
        &state,
        &["autostart", "enable", "--exec", "/usr/bin/nidvue-ui"],
    ));
    let text = std::fs::read_to_string(entry_path(&config_home)).unwrap();
    assert!(text.starts_with("[Desktop Entry]\n"), "{text}");
    assert!(text.contains("Type=Application\n"), "{text}");
    assert!(text.contains("Exec=/usr/bin/nidvue-ui\n"), "{text}");

    let status = json(&run(&config_home, &state, &["--json", "autostart", "status"]));
    assert_eq!(status["enabled"], true);
    assert_eq!(status["exec"], "/usr/bin/nidvue-ui");
    assert_eq!(
        status["path"].as_str().unwrap(),
        entry_path(&config_home).to_str().unwrap()
    );

    stdout(&run(&config_home, &state, &["autostart", "disable"]));
    assert!(!entry_path(&config_home).exists());
    let again = stdout(&run(&config_home, &state, &["autostart", "disable"]));
    assert!(again.contains("already off"), "{again}");
}

#[test]
fn enabling_twice_leaves_one_entry_and_the_default_command_is_the_app() {
    let dir = tempfile::tempdir().unwrap();
    let config_home = dir.path().join("config");
    let state = dir.path().join("state");

    stdout(&run(&config_home, &state, &["autostart", "enable"]));
    stdout(&run(&config_home, &state, &["autostart", "enable"]));

    let autostart_dir = config_home.join("autostart");
    let entries: Vec<_> = std::fs::read_dir(&autostart_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1, "one entry, not one per call: {entries:?}");

    let text = std::fs::read_to_string(entry_path(&config_home)).unwrap();
    let exec = text
        .lines()
        .find_map(|line| line.strip_prefix("Exec="))
        .expect("the entry names a command");
    assert!(
        exec.trim_matches('"').ends_with("nidvue-ui"),
        "the default command is the app, got {exec:?}"
    );
}
