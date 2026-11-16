// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Configuration.
//!
//! Every value has a documented default here, a CLI flag in `nidvue-cli`, and
//! a control in the UI. The file is TOML; the hot-reloadable subset is
//! [`HotConfig`].

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{NidvueError, Result};
use crate::types::ConflictPolicy;
use crate::{DEFAULT_DEVICE, DEFAULT_MOUNTPOINT};

/// Which eviction policy the cache uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvictionPolicy {
    /// Segmented LRU with a frequency scan (default).
    LruK,
    /// Plain least-recently-used.
    Lru,
    /// Never evict (the hard cap is then a hard failure).
    None,
}

/// One bidirectional sync pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SyncPair {
    /// User-visible name, unique in the configuration.
    pub name: String,
    /// Local directory.
    pub local: PathBuf,
    /// Remote directory inside the mountpoint; empty means the mountpoint root.
    #[serde(default)]
    pub remote: String,
    /// Pause this pair without deleting it.
    #[serde(default)]
    pub paused: bool,
    /// Include globs; empty means everything.
    #[serde(default)]
    pub include: Vec<String>,
    /// Exclude globs, applied after `include`.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Per-pair conflict policy; `None` inherits the global one.
    #[serde(default)]
    pub conflict_policy: Option<ConflictPolicy>,
}

/// Cache configuration. Hot-reloadable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct CacheConfig {
    /// Hard cap on blob bytes on disk.
    pub max_size_bytes: u64,
    /// Never fill the disk below this free-space reserve.
    pub min_free_disk_bytes: u64,
    /// Eviction policy.
    pub eviction_policy: EvictionPolicy,
    /// Prefetch directory listings and detected sequential reads.
    pub prefetch: bool,
    /// Readahead window for sequential reads.
    pub readahead_bytes: u64,
    /// Start evicting above this fraction of `max_size_bytes`.
    pub high_watermark: f64,
    /// Stop evicting below this fraction of `max_size_bytes`.
    pub low_watermark: f64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 20 * 1024 * 1024 * 1024,
            min_free_disk_bytes: 2 * 1024 * 1024 * 1024,
            eviction_policy: EvictionPolicy::LruK,
            prefetch: true,
            readahead_bytes: 4 * 1024 * 1024,
            high_watermark: 0.90,
            low_watermark: 0.80,
        }
    }
}

/// Sync engine configuration. Partly hot-reloadable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct SyncConfig {
    /// Configured pairs.
    pub pairs: Vec<SyncPair>,
    /// Minimum remote poll interval; the scheduler backs off above it.
    pub poll_interval_ms: u64,
    /// Maximum remote poll interval after repeated empty polls.
    pub max_poll_interval_ms: u64,
    /// Safety-net full rescan interval (0 disables).
    pub full_scan_interval_ms: u64,
    /// Global conflict policy.
    pub conflict_policy: ConflictPolicy,
    /// Worker pool size for transfers and hashing.
    pub workers: usize,
    /// Aggregate bandwidth cap in bytes/second (0 = unlimited).
    pub bandwidth_limit_bps: u64,
    /// Filesystem event debounce.
    pub debounce_ms: u64,
    /// Window in which a delete+create is treated as a move.
    pub rename_window_ms: u64,
    /// Refuse files larger than this (0 = no limit).
    pub max_file_size_bytes: u64,
    /// Ask for confirmation above this many deletions in one pass.
    pub bulk_delete_threshold: u64,
    /// Local trash retention in days (0 disables the trash).
    pub trash_retention_days: u64,
    /// Pause everything.
    pub paused: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            pairs: Vec::new(),
            poll_interval_ms: 5_000,
            max_poll_interval_ms: 60_000,
            full_scan_interval_ms: 6 * 60 * 60 * 1000,
            conflict_policy: ConflictPolicy::KeepBoth,
            workers: 4,
            bandwidth_limit_bps: 0,
            debounce_ms: 500,
            rename_window_ms: 5_000,
            max_file_size_bytes: 0,
            bulk_delete_threshold: 100,
            trash_retention_days: 30,
            paused: false,
        }
    }
}

/// Mount configuration. Hot-reloadable except `mountpoint`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct MountConfig {
    /// Where the filesystem appears.
    pub mountpoint: PathBuf,
    /// Read-only mount.
    pub read_only: bool,
    /// Pass `allow_other` to FUSE (requires `user_allow_other`).
    pub allow_other: bool,
    /// Kernel attribute cache TTL.
    pub attr_ttl_ms: u64,
    /// Kernel entry cache TTL.
    pub entry_ttl_ms: u64,
    /// Serve `EIO` for writes only after the spool is full (never hang).
    pub durable_spool_bytes: u64,
}

impl Default for MountConfig {
    fn default() -> Self {
        Self {
            mountpoint: PathBuf::from("/tmp/nidvue"),
            read_only: false,
            allow_other: false,
            attr_ttl_ms: 1_000,
            entry_ttl_ms: 1_000,
            durable_spool_bytes: 1024 * 1024 * 1024,
        }
    }
}

/// Network and Jottacloud endpoint configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct NetworkConfig {
    /// JFS base URL.
    pub base_url: String,
    /// Allocate API base URL.
    pub allocate_url: String,
    /// Jottacloud device name.
    pub device: String,
    /// Jottacloud mountpoint name.
    pub mountpoint: String,
    /// Steady-state concurrent requests.
    pub max_concurrency: u32,
    /// Floor the AIMD controller may reduce to.
    pub min_concurrency: u32,
    /// Token bucket refill rate.
    pub requests_per_second: f64,
    /// Token bucket burst.
    pub burst: u32,
    /// Per-request timeout.
    pub timeout_ms: u64,
    /// User agent.
    pub user_agent: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            base_url: "https://jfs.jottacloud.com/jfs".into(),
            allocate_url: "https://api.jottacloud.com/files/v1".into(),
            device: DEFAULT_DEVICE.into(),
            mountpoint: DEFAULT_MOUNTPOINT.into(),
            max_concurrency: 8,
            min_concurrency: 1,
            requests_per_second: 20.0,
            burst: 20,
            timeout_ms: 60_000,
            user_agent: format!("nidvue-drive/{}", crate::VERSION),
        }
    }
}

/// Logging and telemetry configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct LogConfig {
    /// `error`, `warn`, `info`, `debug` or `trace`.
    pub level: String,
    /// Emit JSON lines instead of human-readable text.
    pub json: bool,
    /// Rotating log directory (empty disables file logging).
    pub dir: PathBuf,
    /// Keep this many rotated files.
    pub max_files: usize,
    /// Optional OTLP endpoint, e.g. `http://127.0.0.1:4318`.
    pub otel_endpoint: Option<String>,
    /// Seconds between OTLP exports.
    pub otel_interval_s: u64,
    /// Optional Prometheus bind address, e.g. `127.0.0.1:9464`.
    pub prometheus_bind: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
            json: false,
            dir: PathBuf::new(),
            max_files: 7,
            otel_endpoint: None,
            otel_interval_s: 15,
            prometheus_bind: None,
        }
    }
}

/// Where credentials live.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SecretBackend {
    /// Prefer the OS keyring, fall back to the encrypted file and say why.
    #[default]
    Auto,
    /// The encrypted file only; never touches the platform store.
    File,
    /// The OS keyring only; fails when it is unavailable instead of downgrading.
    Keyring,
}

impl fmt::Display for SecretBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Auto => "auto",
            Self::File => "file",
            Self::Keyring => "keyring",
        })
    }
}

impl std::str::FromStr for SecretBackend {
    type Err = NidvueError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "auto" => Ok(Self::Auto),
            "file" => Ok(Self::File),
            "keyring" => Ok(Self::Keyring),
            other => Err(NidvueError::Config(format!(
                "secret.backend must be auto, file or keyring, not {other:?}"
            ))),
        }
    }
}

/// Credential storage configuration.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(default)]
pub struct SecretConfig {
    /// `auto` (default), `file` or `keyring`.
    pub backend: SecretBackend,
}

/// UI preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct UiConfig {
    /// `system`, `dark` or `light`.
    pub theme: String,
    /// BCP-47 language tag.
    pub language: String,
    /// Start minimised to the tray.
    pub start_minimised: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            language: "en".into(),
            start_minimised: false,
        }
    }
}

/// The hot-reloadable subset, pushed to the UI on change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HotConfig {
    /// Cache settings.
    pub cache: CacheConfig,
    /// Sync settings (minus pairs, which need a restart of the pair loop).
    pub sync: SyncConfig,
    /// Log level.
    pub log_level: String,
}

/// The whole configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, schemars::JsonSchema)]
#[serde(default)]
pub struct AppConfig {
    /// Root state directory; every other default path is below it.
    pub state_dir: PathBuf,
    /// Cache.
    pub cache: CacheConfig,
    /// Sync.
    pub sync: SyncConfig,
    /// Mount.
    pub mount: MountConfig,
    /// Network.
    pub network: NetworkConfig,
    /// Logging.
    pub log: LogConfig,
    /// Credential storage.
    pub secret: SecretConfig,
    /// UI.
    pub ui: UiConfig,
}

impl AppConfig {
    /// The configuration file path for a state directory.
    pub fn path_in(state_dir: &Path) -> PathBuf {
        state_dir.join("config.toml")
    }

    /// Load from a state directory, or return defaults when absent.
    pub fn load_or_default(state_dir: &Path) -> Result<Self> {
        let path = Self::path_in(state_dir);
        if !path.exists() {
            return Ok(Self {
                state_dir: state_dir.to_path_buf(),
                ..Default::default()
            });
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| NidvueError::Config(format!("{}: {e}", path.display())))?;
        let mut cfg: AppConfig = toml::from_str(&text)
            .map_err(|e| NidvueError::Config(format!("{}: {e}", path.display())))?;
        cfg.state_dir = state_dir.to_path_buf();
        cfg.validate()?;
        Ok(cfg)
    }

    /// Write the configuration atomically.
    pub fn save(&self, state_dir: &Path) -> Result<()> {
        self.validate()?;
        std::fs::create_dir_all(state_dir)
            .map_err(|e| NidvueError::Config(format!("{}: {e}", state_dir.display())))?;
        let text = toml::to_string_pretty(self)
            .map_err(|e| NidvueError::Config(format!("serialise: {e}")))?;
        let path = Self::path_in(state_dir);
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text).map_err(|e| NidvueError::io(&tmp, e))?;
        std::fs::rename(&tmp, &path).map_err(|e| NidvueError::io(&path, e))?;
        Ok(())
    }

    /// The database directory.
    pub fn db_dir(&self) -> PathBuf {
        self.state_dir.join("db")
    }

    /// The blob cache directory.
    pub fn blobs_dir(&self) -> PathBuf {
        self.state_dir.join("cache").join("blobs")
    }

    /// The write-back spool directory.
    pub fn spool_dir(&self) -> PathBuf {
        self.state_dir.join("cache").join("spool")
    }

    /// The local trash directory.
    pub fn trash_dir(&self) -> PathBuf {
        self.state_dir.join("trash")
    }

    /// The encrypted credential file.
    pub fn credentials_file(&self) -> PathBuf {
        self.state_dir.join("credentials.enc")
    }

    /// Validate cross-field invariants.
    pub fn validate(&self) -> Result<()> {
        let c = &self.cache;
        if c.max_size_bytes == 0 {
            return Err(NidvueError::Config(
                "cache.max_size_bytes must be > 0".into(),
            ));
        }
        if !(0.0..=1.0).contains(&c.low_watermark) || !(0.0..=1.0).contains(&c.high_watermark) {
            return Err(NidvueError::Config(
                "cache watermarks must be within 0..=1".into(),
            ));
        }
        if c.low_watermark >= c.high_watermark {
            return Err(NidvueError::Config(
                "cache.low_watermark must be below cache.high_watermark".into(),
            ));
        }
        if self.sync.workers == 0 {
            return Err(NidvueError::Config("sync.workers must be > 0".into()));
        }
        if self.sync.poll_interval_ms == 0 {
            return Err(NidvueError::Config(
                "sync.poll_interval_ms must be > 0".into(),
            ));
        }
        if self.sync.max_poll_interval_ms < self.sync.poll_interval_ms {
            return Err(NidvueError::Config(
                "sync.max_poll_interval_ms must be >= sync.poll_interval_ms".into(),
            ));
        }
        let n = &self.network;
        if n.min_concurrency == 0 || n.min_concurrency > n.max_concurrency {
            return Err(NidvueError::Config(
                "network.min_concurrency must be >= 1 and <= max_concurrency".into(),
            ));
        }
        if !n.requests_per_second.is_finite() || n.requests_per_second <= 0.0 {
            return Err(NidvueError::Config(
                "network.requests_per_second must be > 0".into(),
            ));
        }
        let mut names = std::collections::HashSet::new();
        for p in &self.sync.pairs {
            if p.name.trim().is_empty() {
                return Err(NidvueError::Config(
                    "sync pair name must not be empty".into(),
                ));
            }
            if !names.insert(p.name.as_str()) {
                return Err(NidvueError::Config(format!(
                    "duplicate sync pair: {}",
                    p.name
                )));
            }
            for glob in p.include.iter().chain(p.exclude.iter()) {
                globset::Glob::new(glob).map_err(|e| {
                    NidvueError::Config(format!("pair {}: bad glob {glob:?}: {e}", p.name))
                })?;
            }
        }
        Ok(())
    }

    /// The hot-reloadable projection.
    pub fn hot(&self) -> HotConfig {
        HotConfig {
            cache: self.cache.clone(),
            sync: self.sync.clone(),
            log_level: self.log.level.clone(),
        }
    }

    /// Read one value by dotted key, e.g. `sync.poll_interval_ms` or
    /// `sync.pairs.docs.exclude`.
    ///
    /// A segment addresses an object field, a list index, or an element of a
    /// list of objects by its `name`. The value comes back as JSON, so a list
    /// is an array and a number is a number.
    pub fn get_path(&self, key: &str) -> Result<serde_json::Value> {
        let segs = key_segments(key)?;
        let doc = self.to_json()?;
        let mut node = &doc;
        for seg in &segs {
            node = descend(node, seg, key)?;
        }
        Ok(node.clone())
    }

    /// Set one value by dotted key. The text is parsed as JSON when it parses
    /// (numbers, booleans, lists, objects, quoted strings) and taken as a
    /// string otherwise. The result is validated before it replaces `self`.
    pub fn set_path(&mut self, key: &str, raw: &str) -> Result<()> {
        let segs = key_segments(key)?;
        if segs[0] == "state_dir" {
            return Err(NidvueError::Config(
                "state_dir comes from --state, not from config".into(),
            ));
        }
        let value: serde_json::Value = serde_json::from_str(raw)
            .unwrap_or_else(|_| serde_json::Value::String(raw.to_string()));
        let mut doc = self.to_json()?;
        assign(&mut doc, &segs, value, key)?;
        let next: AppConfig =
            serde_json::from_value(doc).map_err(|e| NidvueError::Config(format!("{key}: {e}")))?;
        next.validate()?;
        *self = next;
        Ok(())
    }

    /// Every leaf key with its current value, for `config list`.
    pub fn leaves(&self) -> Result<Vec<(String, serde_json::Value)>> {
        let mut out = Vec::new();
        collect_leaves("", &self.to_json()?, &mut out);
        Ok(out)
    }

    fn to_json(&self) -> Result<serde_json::Value> {
        serde_json::to_value(self)
            .map_err(|e| NidvueError::Internal(format!("config to json: {e}")))
    }
}

fn key_segments(key: &str) -> Result<Vec<&str>> {
    let segs: Vec<&str> = key.split('.').filter(|s| !s.is_empty()).collect();
    if segs.is_empty() {
        return Err(NidvueError::Config("config key is empty".into()));
    }
    Ok(segs)
}

fn descend<'a>(node: &'a serde_json::Value, seg: &str, key: &str) -> Result<&'a serde_json::Value> {
    match node {
        serde_json::Value::Object(map) => map
            .get(seg)
            .ok_or_else(|| NidvueError::Config(format!("unknown config key {key:?}"))),
        serde_json::Value::Array(items) => {
            let index = resolve_index(items, seg, key)?;
            Ok(&items[index])
        }
        _ => Err(NidvueError::Config(format!(
            "{key:?}: {seg:?} is not a field of a scalar"
        ))),
    }
}

fn assign(
    node: &mut serde_json::Value,
    segs: &[&str],
    value: serde_json::Value,
    key: &str,
) -> Result<()> {
    let Some((head, rest)) = segs.split_first() else {
        *node = value;
        return Ok(());
    };
    match node {
        serde_json::Value::Object(map) => {
            let child = map
                .get_mut(*head)
                .ok_or_else(|| NidvueError::Config(format!("unknown config key {key:?}")))?;
            assign(child, rest, value, key)
        }
        serde_json::Value::Array(items) => {
            let index = resolve_index(items, head, key)?;
            assign(&mut items[index], rest, value, key)
        }
        _ => Err(NidvueError::Config(format!(
            "{key:?}: {head:?} is not a field of a scalar"
        ))),
    }
}

fn resolve_index(items: &[serde_json::Value], seg: &str, key: &str) -> Result<usize> {
    if let Ok(index) = seg.parse::<usize>() {
        if index < items.len() {
            return Ok(index);
        }
    }
    items
        .iter()
        .position(|item| item.get("name").and_then(|n| n.as_str()) == Some(seg))
        .ok_or_else(|| NidvueError::Config(format!("{key:?}: no entry named {seg:?}")))
}

fn collect_leaves(
    prefix: &str,
    node: &serde_json::Value,
    out: &mut Vec<(String, serde_json::Value)>,
) {
    match node {
        serde_json::Value::Object(map) => {
            for (name, child) in map {
                let key = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                collect_leaves(&key, child, out);
            }
        }
        serde_json::Value::Array(items) if !items.is_empty() => {
            for (index, child) in items.iter().enumerate() {
                let name = child
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| index.to_string());
                collect_leaves(&format!("{prefix}.{name}"), child, out);
            }
        }
        other => out.push((prefix.to_string(), other.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate_and_round_trip() {
        let cfg = AppConfig::default();
        cfg.validate().unwrap();
        let dir = tempfile::tempdir().unwrap();
        cfg.save(dir.path()).unwrap();
        let back = AppConfig::load_or_default(dir.path()).unwrap();
        assert_eq!(back.cache, cfg.cache);
        assert_eq!(back.network.device, "Jotta");
        assert_eq!(back.network.mountpoint, "Archive");
    }

    #[test]
    fn watermarks_are_ordered() {
        let mut cfg = AppConfig::default();
        cfg.cache.low_watermark = 0.95;
        assert!(cfg.validate().is_err());
        cfg.cache.low_watermark = 0.5;
        cfg.cache.high_watermark = 1.5;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn pairs_are_unique_and_globs_valid() {
        let mut cfg = AppConfig::default();
        let pair = SyncPair {
            name: "docs".into(),
            local: PathBuf::from("/tmp/docs"),
            remote: "/Docs".into(),
            paused: false,
            include: vec!["**/*.md".into()],
            exclude: vec![],
            conflict_policy: None,
        };
        cfg.sync.pairs = vec![pair.clone(), pair];
        assert!(cfg.validate().is_err());
        cfg.sync.pairs.truncate(1);
        cfg.sync.pairs[0].include = vec!["[".into()];
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn hot_projection_is_serialisable() {
        let h = AppConfig::default().hot();
        assert!(serde_json::to_string(&h)
            .unwrap()
            .contains("max_size_bytes"));
    }

    fn pair(name: &str) -> SyncPair {
        SyncPair {
            name: name.into(),
            local: PathBuf::from("/tmp/docs"),
            remote: "/Docs".into(),
            paused: false,
            include: vec![],
            exclude: vec![],
            conflict_policy: None,
        }
    }

    #[test]
    fn path_get_and_set_reach_every_value() {
        let mut cfg = AppConfig::default();
        cfg.sync.pairs = vec![pair("docs")];
        assert_eq!(
            cfg.get_path("sync.poll_interval_ms").unwrap(),
            serde_json::json!(5_000)
        );
        cfg.set_path("sync.poll_interval_ms", "15000").unwrap();
        assert_eq!(cfg.sync.poll_interval_ms, 15_000);
        cfg.set_path("ui.theme", "dark").unwrap();
        assert_eq!(cfg.ui.theme, "dark");
        cfg.set_path("sync.paused", "true").unwrap();
        assert!(cfg.sync.paused);
        cfg.set_path("sync.pairs.docs.exclude", r#"["**/node_modules"]"#)
            .unwrap();
        assert_eq!(cfg.sync.pairs[0].exclude, vec!["**/node_modules"]);
        assert_eq!(
            cfg.get_path("sync.pairs.0.exclude").unwrap(),
            serde_json::json!(["**/node_modules"])
        );
    }

    #[test]
    fn path_set_refuses_unknown_keys_and_bad_values() {
        let mut cfg = AppConfig::default();
        cfg.sync.pairs = vec![pair("docs")];
        assert!(cfg.set_path("sync.nope", "1").is_err());
        assert!(cfg.set_path("state_dir", "/tmp/x").is_err());
        assert!(cfg.set_path("sync.pairs.other.paused", "true").is_err());
        assert!(cfg.set_path("sync.pairs.docs.exclude", r#"["["]"#).is_err());
        assert!(cfg.set_path("cache.max_size_bytes", "0").is_err());
        assert!(cfg.get_path("sync.pairs.docs.nope").is_err());
        assert!(cfg.get_path("").is_err());
        assert_eq!(
            cfg.sync.pairs[0].exclude.len(),
            0,
            "a failed set changes nothing"
        );
    }

    #[test]
    fn leaves_name_each_pair_and_its_values() {
        let mut cfg = AppConfig::default();
        cfg.sync.pairs = vec![pair("docs")];
        let leaves = cfg.leaves().unwrap();
        assert!(leaves
            .iter()
            .any(|(k, v)| k == "sync.pairs.docs.remote" && v == &serde_json::json!("/Docs")));
        assert!(leaves.iter().any(|(k, _)| k == "cache.max_size_bytes"));
    }
}

/// Dot-path access to the configuration, for the CLI, the C ABI and the app.
///
/// The mapping is the serialised form of [`AppConfig`], so a key that exists in
/// the file exists here and a key that does not is an error naming the closest
/// match. A list segment is an index or the `name` of one of its entries.
pub mod dot {
    use super::AppConfig;
    use crate::error::{NidvueError, Result};

    /// A configuration tree as JSON, which is what the dot paths walk.
    pub fn tree(config: &AppConfig) -> Result<serde_json::Value> {
        serde_json::to_value(config)
            .map_err(|e| NidvueError::Config(format!("configuration is not serialisable: {e}")))
    }

    /// Read one leaf by dot path.
    pub fn get(config: &AppConfig, key: &str) -> Result<serde_json::Value> {
        let tree = tree(config)?;
        walk(&tree, key)
            .cloned()
            .ok_or_else(|| unknown_key(&tree, key))
    }

    /// Set one leaf by dot path, returning a configuration that still validates.
    ///
    /// The value is parsed against the type already there, so `true` sets a bool and
    /// `10` a number without the caller naming the type.
    pub fn set(config: &AppConfig, key: &str, raw: &str) -> Result<AppConfig> {
        let mut tree = tree(config)?;
        let existing = walk(&tree, key).ok_or_else(|| unknown_key(&tree, key))?;
        let value = coerce(existing, raw)?;
        let suggestion = unknown_key(&tree, key);
        let slot = walk_mut(&mut tree, key).ok_or(suggestion)?;
        *slot = value;
        let updated: AppConfig =
            serde_json::from_value(tree).map_err(|e| NidvueError::Config(format!("{key}: {e}")))?;
        updated.validate()?;
        Ok(updated)
    }

    /// Every leaf, as `(dot path, value)` pairs in document order.
    pub fn list(config: &AppConfig) -> Result<Vec<(String, serde_json::Value)>> {
        let tree = tree(config)?;
        let mut out = Vec::new();
        flatten(&tree, String::new(), &mut out);
        Ok(out)
    }

    /// Every leaf path, for documentation and for the coverage test.
    pub fn keys(config: &AppConfig) -> Result<Vec<String>> {
        Ok(list(config)?.into_iter().map(|(k, _)| k).collect())
    }

    fn flatten(
        value: &serde_json::Value,
        prefix: String,
        out: &mut Vec<(String, serde_json::Value)>,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    flatten(child, path, out);
                }
            }
            serde_json::Value::Array(items) if !items.is_empty() => {
                for (index, child) in items.iter().enumerate() {
                    flatten(child, format!("{prefix}.{index}"), out);
                }
            }
            other => out.push((prefix, other.clone())),
        }
    }

    fn walk<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
        let mut current = value;
        for part in key.split('.').filter(|p| !p.is_empty()) {
            current = match current {
                serde_json::Value::Object(map) => map.get(part)?,
                serde_json::Value::Array(items) => &items[named_or_indexed(items, part)?],
                _ => return None,
            };
        }
        Some(current)
    }

    /// A list segment is an index, or the `name` of one of its entries, so
    /// `sync.pairs.docs.exclude` addresses the pair called `docs`.
    fn named_or_indexed(items: &[serde_json::Value], part: &str) -> Option<usize> {
        if let Ok(index) = part.parse::<usize>() {
            if index < items.len() {
                return Some(index);
            }
        }
        items
            .iter()
            .position(|item| item.get("name").and_then(|n| n.as_str()) == Some(part))
    }

    fn walk_mut<'a>(
        value: &'a mut serde_json::Value,
        key: &str,
    ) -> Option<&'a mut serde_json::Value> {
        let mut current = value;
        for part in key.split('.').filter(|p| !p.is_empty()) {
            current = match current {
                serde_json::Value::Object(map) => map.get_mut(part)?,
                serde_json::Value::Array(items) => {
                    let index = named_or_indexed(items, part)?;
                    items.get_mut(index)?
                }
                _ => return None,
            };
        }
        Some(current)
    }

    fn coerce(existing: &serde_json::Value, raw: &str) -> Result<serde_json::Value> {
        let trimmed = raw.trim();
        let bad = |what: &str| NidvueError::Config(format!("expected {what}, got {raw:?}"));
        match existing {
            serde_json::Value::Bool(_) => trimmed
                .parse::<bool>()
                .map(serde_json::Value::Bool)
                .map_err(|_| bad("true or false")),
            serde_json::Value::Number(number) => {
                if number.is_f64() {
                    trimmed
                        .parse::<f64>()
                        .map(|v| serde_json::json!(v))
                        .map_err(|_| bad("a number"))
                } else if number.is_u64() {
                    trimmed
                        .parse::<u64>()
                        .map(|v| serde_json::json!(v))
                        .map_err(|_| bad("a non-negative integer"))
                } else {
                    trimmed
                        .parse::<i64>()
                        .map(|v| serde_json::json!(v))
                        .map_err(|_| bad("an integer"))
                }
            }
            serde_json::Value::String(_) => {
                if trimmed == "null" {
                    Ok(serde_json::Value::Null)
                } else {
                    Ok(serde_json::Value::String(raw.to_string()))
                }
            }
            serde_json::Value::Null => {
                if trimmed == "null" || trimmed.is_empty() {
                    Ok(serde_json::Value::Null)
                } else {
                    Ok(serde_json::Value::String(raw.to_string()))
                }
            }
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                serde_json::from_str(trimmed)
                    .map_err(|e| NidvueError::Config(format!("{raw:?} is not valid JSON: {e}")))
            }
        }
    }

    fn unknown_key(tree: &serde_json::Value, key: &str) -> NidvueError {
        let mut all = Vec::new();
        flatten(tree, String::new(), &mut all);
        let suggestion = all
            .iter()
            .map(|(path, _)| (path, distance(path, key)))
            .filter(|(path, d)| path.ends_with(key) || *d <= 3)
            .min_by_key(|(_, d)| *d)
            .map(|(path, _)| format!("; did you mean {path}?"))
            .unwrap_or_else(|| "; run `nidvue-cli config list` for every key".to_string());
        NidvueError::Config(format!("unknown configuration key {key:?}{suggestion}"))
    }

    fn distance(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        let mut cur = vec![0usize; b.len() + 1];
        for (i, ca) in a.iter().enumerate() {
            cur[0] = i + 1;
            for (j, cb) in b.iter().enumerate() {
                let cost = usize::from(ca != cb);
                cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        prev[b.len()]
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::path::PathBuf;

        #[test]
        fn every_leaf_is_readable_by_its_own_path() {
            let config = AppConfig::default();
            let keys = keys(&config).unwrap();
            assert!(keys.len() > 40, "only {} keys found", keys.len());
            for key in &keys {
                let value = get(&config, key).unwrap_or_else(|e| panic!("{key}: {e}"));
                assert!(!value.is_object(), "{key} is not a leaf");
            }
        }

        #[test]
        fn documented_keys_are_present() {
            let config = AppConfig::default();
            let keys = keys(&config).unwrap();
            for key in [
                "state_dir",
                "cache.max_size_bytes",
                "cache.eviction_policy",
                "sync.poll_interval_ms",
                "sync.workers",
                "network.max_concurrency",
                "network.requests_per_second",
                "log.level",
                "log.otel_endpoint",
                "log.otel_interval_s",
                "log.prometheus_bind",
                "secret.backend",
                "ui.theme",
            ] {
                assert!(keys.iter().any(|k| k == key), "missing {key}");
            }
        }

        #[test]
        fn set_parses_against_the_existing_type() {
            let config = AppConfig::default();
            let updated = set(&config, "cache.max_size_bytes", "1048576").unwrap();
            assert_eq!(updated.cache.max_size_bytes, 1_048_576);

            let updated = set(&config, "sync.paused", "true").unwrap();
            assert!(updated.sync.paused);

            let updated = set(&config, "cache.high_watermark", "0.95").unwrap();
            assert!((updated.cache.high_watermark - 0.95).abs() < f64::EPSILON);

            let updated = set(&config, "log.level", "debug").unwrap();
            assert_eq!(updated.log.level, "debug");

            let updated = set(&config, "secret.backend", "keyring").unwrap();
            assert_eq!(
                updated.secret.backend,
                crate::config::SecretBackend::Keyring
            );

            let updated = set(&config, "log.prometheus_bind", "127.0.0.1:9464").unwrap();
            assert_eq!(
                updated.log.prometheus_bind.as_deref(),
                Some("127.0.0.1:9464")
            );
            let cleared = set(&updated, "log.prometheus_bind", "null").unwrap();
            assert!(cleared.log.prometheus_bind.is_none());
        }

        #[test]
        fn every_leaf_round_trips_through_set() {
            let config = AppConfig::default();
            for (key, value) in list(&config).unwrap() {
                if key.starts_with("sync.pairs.") {
                    continue;
                }
                let raw = match &value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => "null".to_string(),
                    other => other.to_string(),
                };
                let updated = set(&config, &key, &raw).unwrap_or_else(|e| panic!("{key}: {e}"));
                let after = get(&updated, &key).unwrap();
                assert_eq!(after, value, "{key} did not round-trip");
            }
        }

        #[test]
        fn a_path_buf_value_is_a_string() {
            let config = AppConfig {
                state_dir: PathBuf::from("/var/lib/nidvue"),
                ..Default::default()
            };
            assert_eq!(
                get(&config, "state_dir").unwrap(),
                serde_json::json!("/var/lib/nidvue")
            );
        }

        #[test]
        fn an_unknown_key_names_a_close_one() {
            let config = AppConfig::default();
            let err = get(&config, "cache.max_size_byte").unwrap_err();
            let text = err.to_string();
            assert!(text.contains("unknown configuration key"), "{text}");
            assert!(text.contains("cache.max_size_bytes"), "{text}");

            let err = get(&config, "nonsense.key").unwrap_err();
            assert!(err.to_string().contains("config list"));
        }

        #[test]
        fn a_wrong_type_is_refused() {
            let config = AppConfig::default();
            assert!(set(&config, "cache.max_size_bytes", "lots")
                .unwrap_err()
                .to_string()
                .contains("non-negative integer"));
            assert!(set(&config, "sync.paused", "yes")
                .unwrap_err()
                .to_string()
                .contains("true or false"));
        }

        #[test]
        fn an_invalid_result_is_refused_by_validate() {
            let config = AppConfig::default();
            assert!(set(&config, "cache.max_size_bytes", "0").is_err());
            assert!(set(&config, "cache.low_watermark", "0.99").is_err());
            assert!(set(&config, "sync.workers", "0").is_err());
        }

        #[test]
        fn pairs_are_addressable_by_index() {
            let mut config = AppConfig::default();
            config.sync.pairs.push(crate::config::SyncPair {
                name: "docs".into(),
                local: PathBuf::from("/tmp/docs"),
                remote: "/Docs".into(),
                paused: false,
                include: vec![],
                exclude: vec![],
                conflict_policy: None,
            });
            assert_eq!(
                get(&config, "sync.pairs.0.name").unwrap(),
                serde_json::json!("docs")
            );
            let updated = set(&config, "sync.pairs.0.paused", "true").unwrap();
            assert!(updated.sync.pairs[0].paused);
        }
    }
}
