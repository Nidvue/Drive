// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Nidvue Drive build automation.
//!
//! `cargo xtask ci` is the one command that must stay green: formatting,
//! clippy with warnings denied, the workspace test suite, docs, the licence
//! allowlist, SPDX headers, the secret scan and the DCO check.

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
        "arch" => arch(),
        "spdx" => spdx(),
        "secrets" => secrets(),
        "lfs" => lfs(),
        "dco" => dco(rest.first().map(String::as_str)),
        "ui" => ui(),
        "ui-smoke" => ui_smoke(),
        "ui-frames" => ui_frames(),
        "ffi-smoke" => ffi_smoke(),
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
         arch      verify the crate dependency contract in ARCHITECTURE.md\n\
         spdx      verify the SPDX header on every tracked source file\n\
         secrets   scan tracked files for credential-shaped strings\n\
         lfs       verify every shipped binary type is routed to Git LFS, and no text\n\
         dco       verify Signed-off-by trailers (default: origin/main..HEAD)\n\
         ui        configure and build the Qt/QML application
         ui-smoke  launch the built UI headlessly and assert it initialises
         ui-frames measure the cost of one rendered frame against the 16 ms budget
         ffi-smoke build a C consumer against include/nidvue.h and run it
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

fn run(program: &str, args: &[&str], dir: &Path, what: &str) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("{what}: spawn {program}: {e}"))?;
    if !status.success() {
        return Err(format!("{what} failed"));
    }
    Ok(())
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
    arch()?;
    clippy()?;
    test(&[])?;
    doc()?;
    deny()?;
    spdx()?;
    secrets()?;
    lfs()?;
    dco(None)?;
    plugins()?;
    ffi_smoke()?;
    if skip_ui {
        println!("+ ui build skipped by --skip-ui");
    } else {
        ui()?;
        ui_smoke()?;
        ui_frames()?;
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

/// The crate contract in `ARCHITECTURE.md`, as a test instead of a review note.
///
/// A rule with no command behind it is a wish: this turns "only jotta speaks
/// HTTP" and "core depends on no Nidvue crate" into a gate that fails the
/// build, so a dependency added in a hurry cannot quietly cross a boundary.
fn arch() -> Result<(), String> {
    // Layering: a crate may depend on a lower rank, never on an equal or
    // higher one. The order is the one the architecture diagram draws.
    const RANKS: &[(&str, u8)] = &[
        ("core", 0),
        ("jotta", 1),
        ("cache", 1),
        ("plugin", 1),
        ("telemetry", 1),
        ("vfs", 2),
        ("mount", 3),
        ("sync", 4),
        ("sdk", 5),
        ("ffi", 6),
        ("cli", 6),
    ];
    let rank = |name: &str| RANKS.iter().find(|(n, _)| *n == name).map(|(_, r)| *r);
    const HTTP_CRATES: &[&str] = &["jotta", "telemetry"];
    let mut problems = Vec::new();

    for (name, mine) in RANKS {
        let manifest = workspace_root().join(format!("crates/nidvue-{name}/Cargo.toml"));
        let deps = manifest_deps(&manifest)?;
        for dep in &deps {
            let Some(other) = dep.strip_prefix("nidvue-") else {
                continue;
            };
            match rank(other) {
                Some(theirs) if theirs < *mine => {}
                Some(_) => problems.push(format!(
                    "nidvue-{name} depends on nidvue-{other}, which is at the same or a higher layer"
                )),
                None => problems.push(format!(
                    "nidvue-{name} depends on unknown crate nidvue-{other}"
                )),
            }
        }
        if *name == "core" && deps.iter().any(|d| d.starts_with("nidvue-")) {
            problems.push("nidvue-core depends on another Nidvue crate".to_string());
        }
        if deps.iter().any(|d| d == "reqwest") && !HTTP_CRATES.contains(name) {
            problems.push(format!(
                "nidvue-{name} speaks HTTP; only {HTTP_CRATES:?} may"
            ));
        }
        if deps.iter().any(|d| d == "fuser") && *name != "mount" {
            problems.push(format!(
                "nidvue-{name} touches the kernel; only nidvue-mount may"
            ));
        }
        if deps
            .iter()
            .any(|d| d.starts_with("qt") || d.starts_with("cxx-qt"))
        {
            problems.push(format!("nidvue-{name} depends on Qt; the engine must not"));
        }
    }

    // The C ABI lives in exactly one crate.
    for entry in walk(&workspace_root().join("crates"))? {
        if entry.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&entry).unwrap_or_default();
        if text.contains("extern \"C\"") && !entry.to_string_lossy().contains("nidvue-ffi") {
            problems.push(format!(
                "{} declares extern \"C\"; only nidvue-ffi may",
                entry.display()
            ));
        }
    }

    if problems.is_empty() {
        println!("+ arch: crate contract holds ({} crates)", RANKS.len());
        return Ok(());
    }
    Err(format!(
        "crate contract violated:\n  - {}",
        problems.join("\n  - ")
    ))
}

/// Dependency names of one manifest, across every `*dependencies]` section.
fn manifest_deps(manifest: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(manifest)
        .map_err(|e| format!("read {}: {e}", manifest.display()))?;
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line.ends_with("dependencies]");
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            deps.push(key.trim().to_string());
        }
    }
    Ok(deps)
}

/// Every file under `dir`, recursively, without pulling in a walker crate.
fn walk(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            out.extend(walk(&path)?);
        } else {
            out.push(path);
        }
    }
    Ok(out)
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
fn ffi_smoke() -> Result<(), String> {
    let root = workspace_root();
    run(
        "cargo",
        &["build", "--release", "-p", "nidvue-ffi"],
        &root,
        "cargo build --release -p nidvue-ffi",
    )?;
    let library = root.join("target/release/libnidvue_ffi.a");
    if !library.exists() {
        return Err(format!("{} is missing after the build", library.display()));
    }
    let source = root.join("crates/nidvue-ffi/tests/consumer.c");
    let binary = std::env::temp_dir().join(format!("nidvue-ffi-consumer-{}", std::process::id()));
    let mut link = vec![
        "-std=c11".to_string(),
        "-Wall".to_string(),
        "-Wextra".to_string(),
        "-Werror".to_string(),
        "-o".to_string(),
        binary.display().to_string(),
        source.display().to_string(),
        "-I".to_string(),
        root.join("crates/nidvue-ffi/include").display().to_string(),
        library.display().to_string(),
    ];
    link.extend(platform_link_libraries());
    let args: Vec<&str> = link.iter().map(String::as_str).collect();
    run("cc", &args, &root, "cc the C consumer")?;
    let state = std::env::temp_dir().join(format!("nidvue-ffi-state-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&state);
    let status = Command::new(&binary)
        .arg(&state)
        .status()
        .map_err(|e| format!("run the C consumer: {e}"))?;
    let _ = std::fs::remove_dir_all(&state);
    let _ = std::fs::remove_file(&binary);
    if !status.success() {
        return Err("the C consumer reported a failure".to_string());
    }
    Ok(())
}

fn platform_link_libraries() -> Vec<String> {
    let mut out = vec![
        "-lpthread".to_string(),
        "-ldl".to_string(),
        "-lm".to_string(),
    ];
    if cfg!(target_os = "linux") {
        out.push("-ldbus-1".to_string());
    }
    out
}

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
    // Two settings are read before the first frame, and both are only visible in
    // the window's own diagnostics: the theme and the language.
    std::fs::create_dir_all(&state).map_err(|e| format!("state dir: {e}"))?;
    std::fs::write(
        state.join("config.toml"),
        "[ui]\ntheme = \"dark\"\nlanguage = \"de\"\n",
    )
    .map_err(|e| format!("config.toml: {e}"))?;
    let mut child = Command::new(&binary)
        .arg("--smoke-windows")
        .env("QT_QPA_PLATFORM", "offscreen")
        // Without this, Qt on Linux sends diagnostics to the journal instead of
        // stderr and the check below would see an empty log.
        .env("QT_ASSUME_STDERR_HAS_CONSOLE", "1")
        .env("NIDVUE_STATE", &state)
        // QSettings and the autostart entry are per-user files. Without this the
        // smoke reads the developer's own preferences, so a configured
        // `ui.theme` could never be observed and the run would touch the
        // machine it runs on.
        .env("XDG_CONFIG_HOME", state.join("config"))
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
    let rows = stderr
        .split("smoke: settings rows=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|count| count.parse::<usize>().ok())
        .unwrap_or(0);
    if rows == 0 {
        return Err(format!(
            "the settings editor received no rows from the engine:\n{stderr}"
        ));
    }
    if !stderr.contains("theme: mode=dark") {
        return Err(format!(
            "the configured ui.theme did not reach the first frame:\n{stderr}"
        ));
    }
    if !stderr.contains("i18n: language=de catalog=:/i18n/nidvue_de.qm") {
        return Err(format!(
            "the configured ui.language did not load its catalog:\n{stderr}"
        ));
    }
    if !stderr.contains("smoke: tray available=0 notice=1") {
        return Err(format!(
            "a desktop without a tray must show the notice that closing quits:\n{stderr}"
        ));
    }
    if !stderr.contains("Einstellungen") {
        return Err(format!(
            "the German catalog did not reach the QML window titles:\n{stderr}"
        ));
    }
    if !stderr.contains("smoke: autostart toggle=1") {
        return Err(format!(
            "the settings window has no Start at login toggle:\n{stderr}"
        ));
    }
    let initialised = state.join("db").join("index.sqlite").exists();
    let _ = std::fs::remove_dir_all(&state);
    if !initialised {
        return Err("the UI did not initialise the engine state directory".into());
    }
    println!("+ ui smoke: engine initialised, {rows} settings rows, no QML errors");
    Ok(())
}

/// The UI frame budget from `docs/performance.md`: one frame, rendered, inside
/// 16 ms so a 60 Hz display is never held up by the window.
fn ui_frames() -> Result<(), String> {
    const BUDGET_MS: f64 = 16.0;
    let root = workspace_root();
    let binary = root.join("apps/nidvue-ui/build/nidvue-ui");
    if !binary.exists() {
        return Err(format!(
            "{} is missing; run `cargo xtask ui` first",
            binary.display()
        ));
    }
    let state = std::env::temp_dir().join(format!("nidvue-ui-frames-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&state);
    std::fs::create_dir_all(&state).map_err(|e| format!("state dir: {e}"))?;
    std::fs::write(
        state.join("config.toml"),
        "[ui]\ntheme = \"dark\"\nlanguage = \"de\"\n",
    )
    .map_err(|e| format!("config.toml: {e}"))?;
    let output = Command::new(&binary)
        .arg("--measure-frames")
        .arg("300")
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_ASSUME_STDERR_HAS_CONSOLE", "1")
        .env("NIDVUE_STATE", &state)
        .env("XDG_CONFIG_HOME", state.join("config"))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("launch ui: {e}"))?;
    let _ = std::fs::remove_dir_all(&state);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let line = stderr
        .lines()
        .find(|line| line.contains("frames:"))
        .ok_or_else(|| format!("the UI reported no frames:\n{stderr}"))?;
    if !line.contains("painted=1") {
        return Err(format!(
            "the UI rendered nothing, so the number is not a frame: {line}"
        ));
    }
    let field = |name: &str| -> Result<f64, String> {
        line.split_whitespace()
            .find_map(|part| part.strip_prefix(name))
            .and_then(|value| value.trim_end_matches("ms").parse::<f64>().ok())
            .ok_or_else(|| format!("{name} missing from {line}"))
    };
    let p99 = field("p99=")?;
    if p99 > BUDGET_MS {
        return Err(format!(
            "frame budget missed: p99 {p99:.3} ms > {BUDGET_MS:.0} ms ({line})"
        ));
    }
    println!("+ ui frames: {line} (budget {BUDGET_MS:.0} ms)");
    Ok(())
}

fn package() -> Result<(), String> {
    cmd("cargo", &["build", "--release", "--workspace"])?;
    cmd("bash", &["packaging/linux.sh"])?;
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
fn lfs() -> Result<(), String> {
    let patterns = std::fs::read_to_string(".gitattributes")
        .map_err(|e| format!("lfs: cannot read .gitattributes: {e}"))?;
    let mut routed: Vec<String> = Vec::new();
    for line in patterns.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains("filter=lfs") {
            continue;
        }
        let Some(glob) = line.split_whitespace().next() else {
            continue;
        };
        routed.push(glob.trim_start_matches('*').to_string());
    }
    if routed.is_empty() {
        return Err("lfs: .gitattributes routes nothing to Git LFS".into());
    }

    let tracked = capture("git", &["ls-files"])?;
    let mut problems = Vec::new();
    let mut covered = 0usize;
    for path in tracked.lines().filter(|p| !p.trim().is_empty()) {
        let attr = capture("git", &["check-attr", "filter", "--", path])?;
        let is_lfs = attr.trim_end().ends_with(": lfs");
        if is_lfs {
            covered += 1;
            // A text file routed into LFS is unreadable without a client, and the
            // two example plugins must stay real bytes: their tests load them from
            // the checkout.
            if std::fs::read_to_string(path).is_ok() && !path.ends_with(".wasm") {
                problems.push(format!("{path} is text but routed into Git LFS"));
            }
        }
    }

    // Every media type the repository ships must be covered by a pattern.
    for path in tracked.lines().filter(|p| !p.trim().is_empty()) {
        let lower = path.to_ascii_lowercase();
        let media = [
            ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".mp4", ".mov", ".webm", ".pdf",
            ".zip",
        ]
        .iter()
        .any(|ext| lower.ends_with(ext))
            || lower.ends_with(".tar.gz");
        if !media {
            continue;
        }
        let attr = capture("git", &["check-attr", "filter", "--", path])?;
        if !attr.trim_end().ends_with(": lfs") {
            problems.push(format!("{path} is a shipped binary with no LFS pattern"));
        }
    }

    if problems.is_empty() {
        println!(
            "lfs: {covered} tracked path(s) through Git LFS, {} pattern(s), no text routed in",
            routed.len()
        );
        Ok(())
    } else {
        Err(format!(
            "lfs: {}\n  {}",
            problems.len(),
            problems.join("\n  ")
        ))
    }
}

/// True when `needle` appears followed by at least `body` key-shaped
/// characters.
///
/// A bare prefix is not a credential: `sk-` alone matches a path such as
/// `xtask-ci.log`, which is a false positive that trains people to ignore the
/// gate. Every real key in the table below carries a long body.
fn credential_shaped(text: &str, needle: &str, body: usize) -> bool {
    text.match_indices(needle).any(|(at, _)| {
        text[at + needle.len()..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .count()
            >= body
    })
}

fn secrets() -> Result<(), String> {
    let files = capture("git", &["ls-files"])?;
    let patterns: &[(&str, &str, usize)] = &[
        ("Jotta personal login token", "eyJ3ZWxsX2tub3duX2xpbmsi", 0),
        ("OpenAI-style key", "sk-", 20),
        ("GitHub token", "ghp_", 20),
        ("GitHub fine-grained token", "github_pat_", 20),
        ("AWS access key", "AKIA", 12),
        ("private key", "BEGIN RSA PRIVATE KEY", 0),
        ("private key", "BEGIN OPENSSH PRIVATE KEY", 0),
        ("Jottacloud bearer", "Bearer eyJ", 0),
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
        for (what, needle, body) in patterns {
            if credential_shaped(&text, needle, *body) {
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
    // On a pull request GitHub checks out a synthetic merge commit, and the
    // base it reports may not be in the checkout. Try the narrowest range that
    // resolves, then fall back to the branch, and never judge a commit nobody
    // authored (a synthetic "Merge ..." commit).
    let base = std::env::var("GITHUB_BASE_SHA")
        .ok()
        .filter(|s| !s.is_empty());
    let mut candidates: Vec<String> = Vec::new();
    if let Some(range) = range {
        candidates.push(range.to_string());
    } else {
        if let Some(sha) = &base {
            candidates.push(format!("{sha}..HEAD"));
        }
        candidates.push("origin/main..HEAD".to_string());
        candidates.push("HEAD".to_string());
    }

    let mut resolved: Option<String> = None;
    for candidate in &candidates {
        let out = Command::new("git")
            .args(["rev-list", "--no-merges", candidate])
            .current_dir(workspace_root())
            .output();
        match out {
            Ok(out) if out.status.success() => {
                resolved = Some(String::from_utf8_lossy(&out.stdout).into_owned());
                break;
            }
            Ok(_) => continue,
            Err(_) => {
                println!("dco: not a git checkout, skipped");
                return Ok(());
            }
        }
    }
    let Some(shas) = resolved else {
        return Err(format!("dco: none of these ranges resolve: {candidates:?}"));
    };

    let mut bad = Vec::new();
    let mut records = 0usize;
    for sha in shas.lines().map(str::to_string) {
        let body = capture("git", &["log", "-1", "--format=%B", &sha])?;
        if body.starts_with("Merge ") {
            continue;
        }
        if body.contains("Signed-off-by:") {
            continue;
        }
        if is_generated_record(&sha, body.lines().next().unwrap_or_default())? {
            records += 1;
            continue;
        }
        bad.push(sha);
    }
    if !records.eq(&0) {
        println!(
            "dco: {records} generated record commit(s) carry no trailer: the teamwork engine \
             writes docs/Team records itself and signs none of them (A-0001)"
        );
    }
    if bad.is_empty() {
        println!("dco: every authored commit carries a Signed-off-by trailer");
        Ok(())
    } else {
        Err(format!(
            "dco: missing Signed-off-by in\n  {}",
            bad.join("\n  ")
        ))
    }
}

/// The subject the teamwork engine writes when it renders a project record.
fn record_subject(subject: &str) -> bool {
    subject.starts_with("chore(records):")
}

/// True when every path a commit touched is a committed project record.
fn record_paths(paths: &[&str]) -> bool {
    !paths.is_empty() && paths.iter().all(|p| p.starts_with("docs/Team/"))
}

/// A commit the teamwork engine generated: its subject and nothing outside the
/// committed project records.
///
/// The engine renders `docs/Team/Archive.md` and `docs/Team/Board.md` after every
/// verdict and commits them on its own identity without a trailer, so a sign-off
/// check over `origin/main..HEAD` fails on bookkeeping nobody authored. The
/// exemption is deliberately narrow - an exact subject prefix *and* every path
/// under `docs/Team/` - so a contributor cannot dodge the check by naming a
/// commit `chore(records):` while changing code.
fn is_generated_record(sha: &str, subject: &str) -> Result<bool, String> {
    if !record_subject(subject) {
        return Ok(false);
    }
    let listing = capture("git", &["show", "--name-only", "--format=", sha])?;
    let touched: Vec<&str> = listing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    Ok(record_paths(&touched))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_records_commit_that_touches_records_is_exempt() {
        assert!(record_subject(
            "chore(records): re-render docs/Team/Board.md at the merge"
        ));
        assert!(!record_subject("chore(merge): merge of developer/t-1"));
        assert!(!record_subject("feat(ui): a feature"));
        assert!(record_paths(&["docs/Team/Board.md"]));
        assert!(record_paths(&[
            "docs/Team/Board.md",
            "docs/Team/Archive.md",
            "docs/Team/AuditManifest.json"
        ]));
        assert!(!record_paths(&[
            "docs/Team/Board.md",
            "crates/nidvue-core/src/lib.rs"
        ]));
        assert!(!record_paths(&[]));
        assert!(!is_generated_record("HEAD", "feat(ui): a feature").unwrap());
    }

    #[test]
    fn a_key_shaped_body_is_a_hit_and_a_bare_prefix_is_not() {
        assert!(credential_shaped(
            "OPENAI_KEY=sk-proj-abc123DEF456ghi789JKL012mno345",
            "sk-",
            20
        ));
        assert!(credential_shaped(
            "token ghp_0123456789abcdefghijABCDEFGHIJ012345",
            "ghp_",
            20
        ));
        assert!(!credential_shaped(
            ".pi/Teamwork/Teams/Drive/Agents/CEO/Reports/xtask-ci.log",
            "sk-",
            20
        ));
        assert!(!credential_shaped("sk- short", "sk-", 20));
        assert!(credential_shaped(
            "eyJ3ZWxsX2tub3duX2xpbmsi",
            "eyJ3ZWxsX2tub3duX2xpbmsi",
            0
        ));
    }
}
