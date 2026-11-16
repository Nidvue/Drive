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
}
