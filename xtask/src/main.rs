// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Nidvue Drive build automation.
//!
//! `cargo xtask ci` is the one command that must stay green: formatting,
//! clippy with warnings denied, the workspace test suite, docs, the licence
//! allowlist, SPDX headers, the secret scan and the DCO check.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map(String::as_str).unwrap_or("help");
    let rest: &[String] = args.get(1..).unwrap_or(&[]);
    match run_task(task, rest) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_task(task: &str, rest: &[String]) -> Result<(), String> {
    match task {
        "ci" => ci(rest.iter().any(|a| a == "--skip-ui")),
        "fmt" => fmt(false),
        "fmt-check" => fmt(true),
        "clippy" => clippy(),
        "test" => test(rest),
        "doc" => doc(),
        "deny" => deny(),
        "spdx" => spdx(),
        "secrets" => secrets(),
        "dco" => dco(rest.first().map(String::as_str)),
        "ui" => ui(),
        "ui-smoke" => ui_smoke(),
        "plugins" => plugins(),
        "bench" => bench(),
        "package" => package(),
        "schema" => schema(),
        "clean" => clean(),
        "help" | "--help" | "-h" => {
            help();
            Ok(())
        }
        other => Err(format!("unknown task {other:?}; try `cargo xtask help`")),
    }
}

fn help() {
    println!(
        "cargo xtask <task>\n\
         \n\
         ci        fmt-check, clippy -D warnings, test, doc, deny, spdx, secrets, dco, ui\n\
         fmt       format the workspace\n\
         clippy    clippy with warnings denied, all targets\n\
         test      the workspace test suite (extra args are passed to cargo test)\n\
         doc       rustdoc with warnings denied\n\
         deny      cargo-deny: licences, advisories, bans, sources\n\
         spdx      verify the SPDX header on every tracked source file\n\
         secrets   scan tracked files for credential-shaped strings\n\
         dco       verify Signed-off-by trailers (default: origin/main..HEAD)\n\
         ui        configure and build the Qt/QML application
         ui-smoke  launch the built UI headlessly and assert it initialises
         plugins   build the two example WASM plugins\n\
         bench     run the performance suite\n\
         package   build release binaries and package recipes\n\
         schema    regenerate the JSON schema and C header\n\
         clean     remove build output"
    );
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

fn cmd(program: &str, args: &[&str]) -> Result<(), String> {
    println!("+ {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(workspace_root())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("spawn {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed ({status})", args.join(" ")))
    }
}

fn capture(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .current_dir(workspace_root())
        .output()
        .map_err(|e| format!("spawn {program}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{program} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn ci(skip_ui: bool) -> Result<(), String> {
    fmt(true)?;
    clippy()?;
    test(&[])?;
    doc()?;
    deny()?;
    spdx()?;
    secrets()?;
    dco(None)?;
    plugins()?;
    if skip_ui {
        println!("+ ui build skipped by --skip-ui");
    } else {
        ui()?;
        ui_smoke()?;
    }
    println!("\nxtask ci: green");
    Ok(())
}

fn fmt(check: bool) -> Result<(), String> {
    let mut args = vec!["fmt", "--all"];
    if check {
        args.push("--check");
    }
    cmd("cargo", &args)
}

fn clippy() -> Result<(), String> {
    cmd(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )
}

fn test(extra: &[String]) -> Result<(), String> {
    let mut args = vec!["test", "--workspace", "--all-features"];
    let extra: Vec<&str> = extra.iter().map(String::as_str).collect();
    args.extend(extra);
    cmd("cargo", &args)
}

fn doc() -> Result<(), String> {
    // `-D warnings` catches broken intra-doc links, the common regression.
    std::env::set_var("RUSTDOCFLAGS", "-D warnings");
    cmd(
        "cargo",
        &["doc", "--workspace", "--no-deps", "--all-features"],
    )
}

fn deny() -> Result<(), String> {
    cmd(
        "cargo",
        &["deny", "check", "licenses", "bans", "sources", "advisories"],
    )
}

fn bench() -> Result<(), String> {
    cmd("cargo", &["bench", "--workspace", "--all-features"])
}

fn ui() -> Result<(), String> {
    let root = workspace_root();
    // CMake links the static library; build it first so a clean checkout works.
    cmd("cargo", &["build", "--release", "-p", "nidvue-ffi"])?;
    let app = root.join("apps/nidvue-ui");
    let build = app.join("build");
    std::fs::create_dir_all(&build).map_err(|e| format!("{}: {e}", build.display()))?;
    let status = Command::new("cmake")
        .args([
            "-S",
            &app.display().to_string(),
            "-B",
            &build.display().to_string(),
        ])
        .args(["-DCMAKE_BUILD_TYPE=Release"])
        .current_dir(&root)
        .status()
        .map_err(|e| format!("cmake configure: {e}"))?;
    if !status.success() {
        return Err("cmake configure failed".into());
    }
    let status = Command::new("cmake")
        .args(["--build", &build.display().to_string(), "--parallel"])
        .current_dir(&root)
        .status()
        .map_err(|e| format!("cmake build: {e}"))?;
    if !status.success() {
        return Err("cmake build failed".into());
    }
    Ok(())
}

/// Build the example plugins to wasm32 and copy them beside their manifests.
fn plugins() -> Result<(), String> {
    let root = workspace_root();
    let examples = [
        (
            "plugins/example-hello",
            "nidvue_plugin_hello.wasm",
            "com_example_hello.wasm",
        ),
        (
            "plugins/example-filter",
            "nidvue_plugin_filter.wasm",
            "com_example_filter.wasm",
        ),
    ];
    for (dir, built, target) in examples {
        let status = Command::new("cargo")
            .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
            .current_dir(root.join(dir))
            .status()
            .map_err(|e| format!("build {dir}: {e}"))?;
        if !status.success() {
            return Err(format!("building {dir} failed"));
        }
        let from = root
            .join(dir)
            .join("target/wasm32-unknown-unknown/release")
            .join(built);
        let to = root.join(dir).join(target);
        std::fs::copy(&from, &to).map_err(|e| format!("copy {}: {e}", from.display()))?;
        println!("+ plugin {target}");
    }
    Ok(())
}

/// Launch the built UI headlessly and assert that it initialises the engine.
fn ui_smoke() -> Result<(), String> {
    let root = workspace_root();
    let binary = root.join("apps/nidvue-ui/build/nidvue-ui");
    if !binary.exists() {
        return Err(format!(
            "{} is missing; run `cargo xtask ui` first",
            binary.display()
        ));
    }
    let state = std::env::temp_dir().join(format!("nidvue-ui-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&state);
    let mut child = Command::new(&binary)
        .arg("--smoke-windows")
        .env("QT_QPA_PLATFORM", "offscreen")
        // Without this, Qt on Linux sends diagnostics to the journal instead of
        // stderr and the check below would see an empty log.
        .env("QT_ASSUME_STDERR_HAS_CONSOLE", "1")
        .env("NIDVUE_STATE", &state)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("launch ui: {e}"))?;
    std::thread::sleep(std::time::Duration::from_secs(3));
    let _ = child.kill();
    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait ui: {e}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("qrc:/")
        || stderr.contains("Cannot assign")
        || stderr.contains("is not a type")
    {
        return Err(format!("QML errors:\n{stderr}"));
    }
    let popup = stderr.contains("tray: popup shown");
    let settings = stderr.contains("tray: settings window shown");
    if !popup || !settings {
        return Err(format!(
            "tray windows did not open (popup={popup}, settings={settings}):\n{stderr}"
        ));
    }
    let initialised = state.join("db").join("index.sqlite").exists();
    let _ = std::fs::remove_dir_all(&state);
    if !initialised {
        return Err("the UI did not initialise the engine state directory".into());
    }
    println!("+ ui smoke: engine initialised, no QML errors");
    Ok(())
}

fn package() -> Result<(), String> {
    cmd("cargo", &["build", "--release", "--workspace"])?;
    let root = workspace_root();
    for script in [
        "packaging/linux.sh",
        "packaging/macos.sh",
        "packaging/windows.ps1",
    ] {
        if root.join(script).exists() && script.ends_with(".sh") && script.contains("linux") {
            cmd("bash", &[script])?;
        }
    }
    println!("packaging: see packaging/ and docs/packaging.md");
    Ok(())
}

fn schema() -> Result<(), String> {
    cmd(
        "cargo",
        &[
            "run",
            "-p",
            "nidvue-sdk",
            "--features",
            "schema",
            "--bin",
            "nidvue-schema",
        ],
    )
}

fn clean() -> Result<(), String> {
    cmd("cargo", &["clean"])?;
    let ui_build = workspace_root().join("apps/nidvue-ui/build");
    if ui_build.exists() {
        std::fs::remove_dir_all(&ui_build).map_err(|e| format!("{}: {e}", ui_build.display()))?;
    }
    Ok(())
}

/// Files that must carry an SPDX header.
fn needs_spdx(path: &str) -> bool {
    const EXTS: &[&str] = &[
        "rs", "qml", "js", "cpp", "cc", "c", "h", "hpp", "sh", "ps1", "yml", "yaml", "toml",
        "cmake", "py",
    ];
    const EXEMPT: &[&str] = &[
        "LICENSE",
        "Cargo.lock",
        "TRADEMARK.md",
        "docs/licensing.md",
        "docs/jotta-api.md",
        "docs/deviations.md",
    ];
    if EXEMPT.contains(&path) {
        return false;
    }
    if path.ends_with("CMakeLists.txt") {
        return true;
    }
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTS.contains(&e))
        .unwrap_or(false)
}

fn spdx() -> Result<(), String> {
    let files = capture("git", &["ls-files"])?;
    let mut missing = Vec::new();
    for path in files.lines().filter(|p| needs_spdx(p)) {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let head: String = text.lines().take(4).collect::<Vec<_>>().join("\n");
        if !head.contains("SPDX-License-Identifier: AGPL-3.0-only") {
            missing.push(path.to_string());
        }
        if !text.contains("Copyright (C)") {
            missing.push(format!("{path} (no copyright line)"));
        }
    }
    if missing.is_empty() {
        println!("spdx: every tracked source file carries the header");
        Ok(())
    } else {
        Err(format!(
            "spdx: missing header in\n  {}",
            missing.join("\n  ")
        ))
    }
}

/// Credential-shaped strings that must never be committed.
fn secrets() -> Result<(), String> {
    let files = capture("git", &["ls-files"])?;
    let patterns: &[(&str, &str)] = &[
        ("Jotta personal login token", "eyJ3ZWxsX2tub3duX2xpbmsi"),
        ("OpenAI-style key", "sk-"),
        ("GitHub token", "ghp_"),
        ("GitHub fine-grained token", "github_pat_"),
        ("AWS access key", "AKIA"),
        ("private key", "BEGIN RSA PRIVATE KEY"),
        ("private key", "BEGIN OPENSSH PRIVATE KEY"),
        ("Jottacloud bearer", "Bearer eyJ"),
    ];
    let mut hits = Vec::new();
    for path in files.lines() {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        if path.starts_with("xtask/") {
            // The scanner's own pattern table.
            continue;
        }
        for (what, needle) in patterns {
            if text.contains(needle) {
                hits.push(format!("{path}: {what}"));
            }
        }
    }
    if hits.is_empty() {
        println!("secrets: no credential-shaped strings in tracked files");
        Ok(())
    } else {
        Err(format!(
            "secrets: possible credentials\n  {}",
            hits.join("\n  ")
        ))
    }
}

fn dco(range: Option<&str>) -> Result<(), String> {
    let range = range.unwrap_or("origin/main..HEAD");
    let out = Command::new("git")
        .args(["rev-list", "--no-merges", range])
        .current_dir(workspace_root())
        .output();
    let Ok(out) = out else {
        println!("dco: not a git checkout, skipped");
        return Ok(());
    };
    if !out.status.success() {
        // No origin/main yet (first push): check every commit on the branch.
        return dco(Some("HEAD"));
    }
    let shas: BTreeSet<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    let mut bad = Vec::new();
    for sha in shas {
        let body = capture("git", &["log", "-1", "--format=%B", &sha])?;
        if !body.contains("Signed-off-by:") {
            bad.push(sha);
        }
    }
    if bad.is_empty() {
        println!("dco: every commit carries a Signed-off-by trailer");
        Ok(())
    } else {
        Err(format!(
            "dco: missing Signed-off-by in\n  {}",
            bad.join("\n  ")
        ))
    }
}
