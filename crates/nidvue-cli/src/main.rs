// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Headless control for Nidvue Drive.
//!
//! Every operation the desktop application performs is reachable here, which is
//! what makes `mount` work with no UI running and `status --json` scriptable.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use nidvue_cli::config_dot;
use nidvue_core::config::{AppConfig, SyncPair};
use nidvue_core::error::NidvueError;
use nidvue_core::path::RemotePath;
use nidvue_core::types::ConflictPolicy;
use nidvue_mount::MountOptions;
use nidvue_sdk::Client;

#[derive(Parser, Debug)]
#[command(
    name = "nidvue-cli",
    version,
    about = "Nidvue Drive — unofficial Jottacloud sync and mount client",
    long_about = "Nidvue Drive is an unofficial Jottacloud client (AGPL-3.0-only). \
                  It is not affiliated with Jottacloud AS."
)]
struct Cli {
    /// State directory (default: $XDG_DATA_HOME/nidvue-drive)
    #[arg(long, global = true, env = "NIDVUE_STATE")]
    state: Option<PathBuf>,
    /// Machine-readable output where the command supports it
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Exchange a Personal Login Token for a bearer token
    Login {
        /// Read the token from stdin instead of prompting
        #[arg(long)]
        token_stdin: bool,
        /// Token value (visible in shell history; prefer --token-stdin)
        #[arg(long, hide = true)]
        token: Option<String>,
    },
    /// Forget the stored token
    Logout,
    /// Print engine status
    Status,
    /// Mount the account at a local path (blocks until Ctrl-C)
    Mount {
        /// Where to mount (default: config mountpoint)
        mountpoint: Option<PathBuf>,
        /// Mount read-only
        #[arg(long)]
        read_only: bool,
    },
    /// Manage sync pairs
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },
    /// Diagnose the installation and optionally run the performance budgets
    Doctor {
        /// Run the performance budget measurements
        #[arg(long)]
        bench: bool,
    },
    /// Inspect and resolve conflicts
    Conflicts {
        #[command(subcommand)]
        action: ConflictAction,
    },
    /// Inspect and test plugins
    Plugins {
        #[command(subcommand)]
        action: PluginAction,
    },
    /// Read and write configuration values by dot path
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    /// Print one value, e.g. cache.max_size_bytes
    Get {
        /// Dot path into config.toml
        key: String,
    },
    /// Set one value and save the configuration
    Set {
        /// Dot path into config.toml
        key: String,
        /// New value, parsed against the type already there
        value: String,
    },
    /// Print every key and its value
    List,
    /// Print the configuration file path
    Path,
}

#[derive(Subcommand, Debug)]
enum SyncAction {
    /// Add a pair to the configuration
    Add {
        /// Pair name
        name: String,
        /// Local directory
        local: PathBuf,
        /// Remote directory inside the mountpoint
        remote: String,
    },
    /// List configured pairs
    List,
    /// Run one pass (all pairs, or one by name)
    Run { name: Option<String> },
}

#[derive(Subcommand, Debug)]
enum ConflictAction {
    /// List open conflicts
    List,
    /// Resolve one conflict
    Resolve {
        /// Conflict id
        id: String,
        /// keep_local, keep_remote or keep_both
        resolution: String,
    },
}

#[derive(Subcommand, Debug)]
enum PluginAction {
    /// List installed plugins
    List,
    /// Install a plugin directory (copies it under <state>/plugins)
    Install { dir: PathBuf },
    /// Load a plugin and call its declared hooks with sample payloads
    Test { dir: PathBuf },
}

fn state_dir(cli: &Cli) -> PathBuf {
    if let Some(dir) = &cli.state {
        return dir.clone();
    }
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(dir).join("nidvue-drive");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/nidvue-drive");
    }
    PathBuf::from("nidvue-drive")
}

fn init_logging() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_env("NIDVUE_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,nidvue_jotta=warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

#[tokio::main]
async fn main() {
    init_logging();
    let cli = Cli::parse();
    let state = state_dir(&cli);
    match run(&cli, &state).await {
        Ok(()) => {}
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(e.exit_code());
        }
    }
}

async fn run(cli: &Cli, state: &Path) -> Result<(), NidvueError> {
    match &cli.command {
        Command::Login { token_stdin, token } => {
            let value = match token {
                Some(t) => t.clone(),
                None if *token_stdin => {
                    let mut buf = String::new();
                    std::io::stdin()
                        .read_to_string(&mut buf)
                        .map_err(|e| NidvueError::io("<stdin>", e))?;
                    buf.trim().to_string()
                }
                None => {
                    return Err(NidvueError::Config(
                        "pass --token-stdin (recommended) or --token".into(),
                    ))
                }
            };
            let client = Client::open(state).await?;
            let username = client.login(&value).await?;
            println!("logged in as {username}");
            println!(
                "token stored encrypted in {}",
                state.join("credentials.enc").display()
            );
            Ok(())
        }
        Command::Logout => {
            let client = Client::open(state).await?;
            client.logout().await?;
            println!("logged out");
            Ok(())
        }
        Command::Status => {
            let client = Client::open(state).await?;
            let status = client.status().await?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&status).unwrap_or_default()
                );
            } else {
                println!("Nidvue Drive {}", status.version);
                println!(
                    "account:   {}",
                    status
                        .username
                        .clone()
                        .unwrap_or_else(|| "not logged in".into())
                );
                println!("health:    {:?}", status.health);
                println!("indexed:   {} entries", status.index_entries);
                println!(
                    "cache:     {} / {} bytes, {} blobs ({} pinned, {} dirty)",
                    status.cache.bytes,
                    client.config().cache.max_size_bytes,
                    status.cache.blobs,
                    status.cache.pinned,
                    status.cache.dirty
                );
                println!("conflicts: {}", status.conflicts_open);
                if let Some(q) = status.quota {
                    println!("quota:     {} / {} bytes", q.usage, q.quota);
                }
            }
            Ok(())
        }
        Command::Mount {
            mountpoint,
            read_only,
        } => {
            let client = Client::open(state).await?;
            if !client.is_logged_in().await {
                return Err(NidvueError::Auth("log in first: nidvue-cli login".into()));
            }
            let target = mountpoint
                .clone()
                .unwrap_or_else(|| client.config().mount.mountpoint.clone());
            let options = MountOptions {
                read_only: *read_only,
                ..Default::default()
            };
            let handle = client.mount(&target, options)?;
            println!("mounted at {}", handle.mountpoint().display());
            println!("press Ctrl-C to unmount");
            let _ = tokio::signal::ctrl_c().await;
            drop(handle);
            println!("unmounted");
            Ok(())
        }
        Command::Sync { action } => match action {
            SyncAction::Add {
                name,
                local,
                remote,
            } => {
                let mut config = AppConfig::load_or_default(state)?;
                if config.sync.pairs.iter().any(|p| &p.name == name) {
                    return Err(NidvueError::Config(format!("pair {name} already exists")));
                }
                RemotePath::parse(remote)?;
                config.sync.pairs.push(SyncPair {
                    name: name.clone(),
                    local: local.clone(),
                    remote: remote.clone(),
                    paused: false,
                    include: vec![],
                    exclude: vec![],
                    conflict_policy: None,
                });
                config.save(state)?;
                println!("added pair {name}: {} ⇅ {remote}", local.display());
                Ok(())
            }
            SyncAction::List => {
                let config = AppConfig::load_or_default(state)?;
                if config.sync.pairs.is_empty() {
                    println!("no pairs configured");
                }
                for pair in &config.sync.pairs {
                    println!(
                        "{:<16} {} ⇅ {}{}",
                        pair.name,
                        pair.local.display(),
                        pair.remote,
                        if pair.paused { " (paused)" } else { "" }
                    );
                }
                Ok(())
            }
            SyncAction::Run { name } => {
                let client = Client::open(state).await?;
                if !client.is_logged_in().await {
                    return Err(NidvueError::Auth("log in first: nidvue-cli login".into()));
                }
                let reports = client.sync_now().await?;
                for (i, report) in reports.iter().enumerate() {
                    let pair = &client.sync_pairs()[i];
                    if let Some(only) = name {
                        if &pair.name != only {
                            continue;
                        }
                    }
                    println!(
                        "{}: scanned {} planned {} applied {} conflicts {} failed {}",
                        pair.name,
                        report.scanned,
                        report.planned,
                        report.applied,
                        report.conflicts,
                        report.failed.len()
                    );
                    for (path, error) in &report.failed {
                        eprintln!("  {path}: {error}");
                    }
                }
                Ok(())
            }
        },
        Command::Doctor { bench } => doctor(state, *bench, cli.json).await,
        Command::Conflicts { action } => match action {
            ConflictAction::List => {
                let client = Client::open(state).await?;
                let conflicts = client.conflicts()?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&conflicts).unwrap_or_default()
                    );
                } else if conflicts.is_empty() {
                    println!("no open conflicts");
                } else {
                    for c in conflicts {
                        println!("{}  {}  → {}", c.id, c.path, c.conflict_path);
                    }
                }
                Ok(())
            }
            ConflictAction::Resolve { id, resolution } => {
                let policy = ConflictPolicy::parse(resolution).ok_or_else(|| {
                    NidvueError::Config(format!("unknown resolution {resolution:?}"))
                })?;
                let client = Client::open(state).await?;
                client.resolve_conflict(id, policy)?;
                println!("resolved {id} as {}", policy.as_str());
                Ok(())
            }
        },
        Command::Plugins { action } => match action {
            PluginAction::List => {
                let dir = state.join("plugins");
                let mut found = 0;
                if dir.is_dir() {
                    for entry in std::fs::read_dir(&dir).map_err(|e| NidvueError::io(&dir, e))? {
                        let entry = entry.map_err(|e| NidvueError::io(&dir, e))?;
                        if entry.path().join("plugin.toml").exists() {
                            match nidvue_plugin::PluginManifest::load(&entry.path()) {
                                Ok(m) => {
                                    found += 1;
                                    println!(
                                        "{} {}  {}  [{}]",
                                        m.id,
                                        m.version,
                                        m.name,
                                        m.capabilities.hooks.join(", ")
                                    );
                                }
                                Err(e) => eprintln!("{}: {e}", entry.path().display()),
                            }
                        }
                    }
                }
                if found == 0 {
                    println!("no plugins installed in {}", dir.display());
                }
                Ok(())
            }
            PluginAction::Install { dir } => {
                let manifest = nidvue_plugin::PluginManifest::load(dir)?;
                let target = state.join("plugins").join(&manifest.id);
                std::fs::create_dir_all(&target).map_err(|e| NidvueError::io(&target, e))?;
                for entry in std::fs::read_dir(dir).map_err(|e| NidvueError::io(dir, e))? {
                    let entry = entry.map_err(|e| NidvueError::io(dir, e))?;
                    let name = entry.file_name();
                    std::fs::copy(entry.path(), target.join(&name))
                        .map_err(|e| NidvueError::io(entry.path(), e))?;
                }
                println!(
                    "installed {} {} at {}",
                    manifest.id,
                    manifest.version,
                    target.display()
                );
                Ok(())
            }
            PluginAction::Test { dir } => {
                let mut host = nidvue_plugin::PluginHost::new()?;
                let id = host.load(dir)?;
                let hooks = host
                    .plugin_mut(&id)
                    .map(|p| p.manifest.capabilities.hooks.clone())
                    .unwrap_or_default();
                for hook in hooks {
                    let payload = match hook.as_str() {
                        "path_filter" => r#"{"path":"/Archive/Documents/a.txt","kind":"file"}"#,
                        "conflict_resolve" => {
                            r#"{"path":"/Archive/a","local_hash":"a","remote_hash":"b"}"#
                        }
                        _ => "{}",
                    };
                    match host.plugin_mut(&id).map(|p| p.call_hook(&hook, payload)) {
                        Some(Ok(response)) => println!("{hook}: {response}"),
                        Some(Err(e)) => println!("{hook}: FAILED {e}"),
                        None => {}
                    }
                }
                Ok(())
            }
        },
        Command::Config { action } => match action {
            ConfigAction::Path => {
                println!("{}", AppConfig::path_in(state).display());
                Ok(())
            }
            ConfigAction::Get { key } => {
                let config = AppConfig::load_or_default(state)?;
                let value = config_dot::get(&config, key)?;
                match &value {
                    serde_json::Value::String(text) => println!("{text}"),
                    other => println!("{other}"),
                }
                Ok(())
            }
            ConfigAction::Set { key, value } => {
                let config = AppConfig::load_or_default(state)?;
                let updated = config_dot::set(&config, key, value)?;
                updated.save(state)?;
                println!("{key} = {value}");
                Ok(())
            }
            ConfigAction::List => {
                let config = AppConfig::load_or_default(state)?;
                let leaves = config_dot::list(&config)?;
                if cli.json {
                    let map: serde_json::Map<String, serde_json::Value> =
                        leaves.into_iter().collect();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::Value::Object(map))
                            .unwrap_or_default()
                    );
                } else {
                    let width = leaves.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
                    for (key, value) in leaves {
                        match value {
                            serde_json::Value::String(text) => println!("{key:<width$}  {text}"),
                            other => println!("{key:<width$}  {other}"),
                        }
                    }
                }
                Ok(())
            }
        },
    }
}

async fn doctor(state: &Path, bench: bool, json: bool) -> Result<(), NidvueError> {
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    let config = AppConfig::load_or_default(state)?;
    checks.push((
        "config".into(),
        config.validate().is_ok(),
        "configuration validates".into(),
    ));
    std::fs::create_dir_all(state).map_err(|e| NidvueError::io(state, e))?;
    let probe = state.join(".doctor-probe");
    let writable = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    checks.push((
        "state".into(),
        writable,
        format!("{} is writable", state.display()),
    ));
    let selection =
        nidvue_core::secret::open_store(config.secret.backend, config.credentials_file());
    checks.push((
        "secret".into(),
        !selection.degraded,
        format!("credentials: {}", selection.describe()),
    ));
    if selection.degraded {
        checks.push((
            "engine".into(),
            false,
            "not opened: the requested credential store is unusable".into(),
        ));
        return report(checks, Vec::new(), json);
    }
    drop(selection);
    let client = match Client::open(state).await {
        Ok(client) => client,
        Err(e) => {
            checks.push(("engine".into(), false, e.to_string()));
            return report(checks, Vec::new(), json);
        }
    };
    checks.push((
        "login".into(),
        client.is_logged_in().await,
        if client.is_logged_in().await {
            "token present"
        } else {
            "run nidvue-cli login"
        }
        .into(),
    ));
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let fuse = Path::new("/dev/fuse").exists();
        checks.push(("fuse".into(), fuse, "/dev/fuse present".into()));
    }
    #[cfg(target_os = "windows")]
    checks.push((
        "winfsp".into(),
        false,
        "WinFSP host is not enabled in this build".into(),
    ));
    let status = client.status().await?;
    checks.push(match &status.prometheus_url {
        Some(url) => ("metrics".into(), true, format!("scrape {url}")),
        None => (
            "metrics".into(),
            true,
            "log.prometheus_bind is unset; no scrape endpoint".into(),
        ),
    });
    checks.push(match &status.otlp_endpoint {
        Some(endpoint) => ("otlp".into(), true, format!("export to {endpoint}")),
        None => (
            "otlp".into(),
            true,
            "log.otel_endpoint is unset; no exporter".into(),
        ),
    });
    let usage = client.cache().usage()?;
    checks.push((
        "cache".into(),
        usage.bytes <= config.cache.max_size_bytes,
        format!("{} / {} bytes", usage.bytes, config.cache.max_size_bytes),
    ));

    let mut bench_lines = Vec::new();
    if bench {
        bench_lines = run_bench(&client)?;
    }

    report(checks, bench_lines, json)
}

type Checks = Vec<(String, bool, String)>;

fn report(checks: Checks, bench_lines: Vec<String>, json: bool) -> Result<(), NidvueError> {
    if json {
        let out = serde_json::json!({
            "checks": checks.iter().map(|(name, ok, detail)| serde_json::json!({
                "name": name, "ok": ok, "detail": detail
            })).collect::<Vec<_>>(),
            "bench": bench_lines,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        for (name, ok, detail) in &checks {
            println!("{} {name}: {detail}", if *ok { "ok  " } else { "FAIL" });
        }
        for line in &bench_lines {
            println!("{line}");
        }
        if checks.iter().any(|(_, ok, _)| !ok) {
            println!("\nsome checks failed");
        }
    }
    Ok(())
}

/// Measure the published budgets. These are the numbers `docs/performance.md`
/// talks about, produced on the machine running the command.
fn run_bench(client: &Client) -> Result<Vec<String>, NidvueError> {
    use nidvue_core::hash::Hash;
    let mut out = Vec::new();
    // Warm cached read of 1 MiB.
    let payload = vec![0x5au8; 1024 * 1024];
    let hash = client.cache().put(&payload)?;
    let mut samples = Vec::with_capacity(200);
    for _ in 0..200 {
        let start = Instant::now();
        let bytes = client.cache().read(&hash)?;
        std::hint::black_box(&bytes);
        samples.push(start.elapsed());
    }
    let p50 = percentile(&mut samples, 0.50);
    out.push(format!(
        "warm read 1 MiB: p50 {:?} (budget ≤ 15 ms) {}",
        p50,
        if p50 <= Duration::from_millis(15) {
            "PASS"
        } else {
            "MISS"
        }
    ));

    // Cached metadata op against a populated index.
    for i in 0..1000 {
        client.index().upsert_entry(&nidvue_cache::EntryRow {
            path: format!("/bench/dir{i}"),
            kind: nidvue_core::types::EntryKind::Dir,
            size: 0,
            mtime_ms: 0,
            hash: None,
            remote_rev: None,
            dirty: false,
        })?;
    }
    let mut meta_samples = Vec::with_capacity(200);
    for _ in 0..200 {
        let start = Instant::now();
        let entries = client.index().list_dir("/bench")?;
        std::hint::black_box(&entries);
        meta_samples.push(start.elapsed());
    }
    let meta_p50 = percentile(&mut meta_samples, 0.50);
    out.push(format!(
        "cached readdir page: p50 {:?} (budget ≤ 2 ms) {}",
        meta_p50,
        if meta_p50 <= Duration::from_millis(2) {
            "PASS"
        } else {
            "MISS"
        }
    ));

    // Hashing throughput, which bounds uploads.
    let start = Instant::now();
    let hashed = Hash::of(&payload);
    std::hint::black_box(&hashed);
    let elapsed = start.elapsed();
    let mbps = 1.0 / elapsed.as_secs_f64().max(f64::MIN_POSITIVE);
    out.push(format!("sha256 1 MiB: {elapsed:?} ({mbps:.0} MiB/s)"));
    client.cache().blobs().remove(&hash)?;
    client.index().remove_blob(hash.as_str())?;
    Ok(out)
}

fn percentile(samples: &mut [Duration], p: f64) -> Duration {
    samples.sort();
    if samples.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((samples.len() - 1) as f64 * p).round() as usize;
    samples[idx]
}
