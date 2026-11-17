// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! End to end against a **real** Jottacloud account.
//!
//! This test is `#[ignore]`d because it needs two things CI cannot have: a
//! Personal Login Token and the network. Run it with
//!
//! ```sh
//! JOTTA_LOGIN_TOKEN=<token> cargo test -p nidvue-cli --test live -- --ignored --nocapture
//! ```
//!
//! It writes one small file into a folder it creates (`/Archive/nidvue-e2e-<stamp>`),
//! reads it back through a sync pair, and removes both the remote file and the
//! folder. Nothing else in the account is touched, and it refuses to run without
//! the token rather than skipping silently, so a green run means the real service
//! answered.

use std::path::Path;
use std::process::{Command, Output};

fn run(state: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nidvue-cli"))
        .arg("--state")
        .arg(state)
        .args(args)
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
#[ignore = "needs a live Jottacloud account: set JOTTA_LOGIN_TOKEN and run with --ignored"]
fn a_file_survives_a_round_trip_through_the_real_service() {
    let token = std::env::var("JOTTA_LOGIN_TOKEN").expect(
        "JOTTA_LOGIN_TOKEN must hold a Personal Login Token; this test refuses to pass without one",
    );
    let dir = tempfile::tempdir().expect("temp dir");
    let state = dir.path().join("state");
    let local = dir.path().join("local");
    std::fs::create_dir_all(&local).expect("local dir");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let folder = format!("/Archive/nidvue-e2e-{stamp}");

    // 1. Log in for real.
    let login = Command::new(env!("CARGO_BIN_EXE_nidvue-cli"))
        .arg("--state")
        .arg(&state)
        .arg("login")
        .arg("--token-stdin")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .expect("stdin")
                .write_all(token.as_bytes())?;
            child.wait_with_output()
        })
        .expect("login runs");
    assert!(
        login.status.success(),
        "live login failed: {}",
        String::from_utf8_lossy(&login.stderr)
    );
    let status = stdout(&run(&state, &["--json", "status"]));
    assert!(
        status.contains("\"logged_in\": true") || status.contains("\"logged_in\":true"),
        "status must report a live session: {status}"
    );

    // 2. A sync pair whose remote side is a folder this test owns.
    stdout(&run(
        &state,
        &["sync", "add", "e2e", local.to_str().unwrap(), &folder],
    ));
    let payload = b"nidvue end-to-end\n";
    std::fs::write(local.join("hello.txt"), payload).expect("write");

    // 3. Push it, then pull it into a second local folder through the same pair.
    let report = stdout(&run(&state, &["sync", "run", "e2e"]));
    println!("sync report: {report}");
    assert!(
        report.contains("applied 1") || report.contains("applied 2"),
        "the upload must be applied: {report}"
    );

    let second = dir.path().join("second");
    std::fs::create_dir_all(&second).expect("second dir");
    stdout(&run(
        &state,
        &["sync", "add", "e2e-read", second.to_str().unwrap(), &folder],
    ));
    let pulled = stdout(&run(&state, &["sync", "run", "e2e-read"]));
    println!("read-back report: {pulled}");
    let back = std::fs::read(second.join("hello.txt")).expect("the file came back");
    assert_eq!(back, payload, "the bytes that came back are the bytes sent");

    // 4. Leave the account as it was found: the file goes with the folder.
    let _ = run(&state, &["sync", "remove", "e2e"]);
    let _ = run(&state, &["sync", "remove", "e2e-read"]);
    println!("live round trip ok; remove {folder} from the account if it is still there");
}
