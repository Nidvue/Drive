// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The stable Nidvue Drive SDK.
//!
//! One [`Client`] owns the account, the cache, the index and the event stream.
//! Everything the desktop application, the CLI and a plugin author needs is
//! reachable from here; nothing below this crate is required to use it.
//!
//! ```no_run
//! # async fn run() -> nidvue_core::Result<()> {
//! use nidvue_sdk::Client;
//! let client = Client::open("/tmp/nidvue-state").await?;
//! let status = client.status().await?;
//! println!("{} files indexed, {} conflicts open", status.index_entries, status.conflicts_open);
//! # Ok(())
//! # }
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use nidvue_cache::{Cache, CacheUsage, ConflictRow, Index};
use nidvue_core::config::{AppConfig, SyncPair};
use nidvue_core::error::{NidvueError, Result};
use nidvue_core::event::{Event, EventEnvelope, HealthState, StatsSnapshot};
use nidvue_core::types::ConflictPolicy;
use nidvue_jotta::auth::AuthManager;
use nidvue_jotta::client::{JottaClient, JottaConfig, Quota};
use nidvue_mount::{MountHandle, MountOptions};
use nidvue_plugin::PluginHost;
use nidvue_sync::{SyncEngine, SyncReport};
use nidvue_vfs::EngineVfs;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub use nidvue_core::config::{CacheConfig, LogConfig, MountConfig, NetworkConfig, SyncConfig};
pub use nidvue_core::conflict::{Conflict, Resolution};
pub use nidvue_core::error::NidvueError as Error;
pub use nidvue_core::path::RemotePath;
pub use nidvue_core::types::{EntryKind, EntryMeta, Version};
pub use nidvue_jotta::xml::{ListStream, RemoteEntry};
pub use nidvue_sync::Action;

/// A point-in-time view of the client, as `status --json` prints it.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Status {
    /// SDK version.
    pub version: String,
    /// True when a token is present.
    pub logged_in: bool,
    /// The account username, when known.
    pub username: Option<String>,
    /// Entries in the metadata index.
    pub index_entries: u64,
    /// Conflicts awaiting a decision.
    pub conflicts_open: u64,
    /// Cache counters.
    pub cache: CacheUsageView,
    /// Overall health.
    pub health: HealthState,
    /// Seconds since the client was opened.
    pub uptime_s: u64,
    /// Account quota, when the server answered.
    pub quota: Option<QuotaView>,
}

/// Cache counters in the status view.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CacheUsageView {
    /// Bytes stored.
    pub bytes: u64,
    /// Blobs stored.
    pub blobs: u64,
    /// Pinned blobs.
    pub pinned: u64,
    /// Blobs not yet drained.
    pub dirty: u64,
}

impl From<CacheUsage> for CacheUsageView {
    fn from(u: CacheUsage) -> Self {
        Self { bytes: u.bytes, blobs: u.blobs, pinned: u.pinned, dirty: u.dirty }
    }
}

/// Account quota in the status view.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema)]
pub struct QuotaView {
    /// Bytes used.
    pub usage: u64,
    /// Bytes allowed.
    pub quota: u64,
}

impl From<Quota> for QuotaView {
    fn from(q: Quota) -> Self {
        Self { usage: q.usage, quota: q.quota }
    }
}

struct Inner {
    config: AppConfig,
    auth: Arc<AuthManager>,
    jotta: Arc<JottaClient>,
    cache: Arc<Cache>,
    index: Arc<Index>,
    events: broadcast::Sender<EventEnvelope>,
    cancel: CancellationToken,
    started: Instant,
}

/// The Nidvue Drive client.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

impl Client {
    /// Open (or create) a client state directory.
    pub async fn open(state_dir: impl Into<PathBuf>) -> Result<Self> {
        let state_dir = state_dir.into();
        let config = AppConfig::load_or_default(&state_dir)?;
        std::fs::create_dir_all(config.db_dir()).map_err(|e| NidvueError::io(config.db_dir(), e))?;
        let cache = Arc::new(Cache::open(
            &config.cache,
            &config.blobs_dir(),
            &config.db_dir().join("index.sqlite"),
        )?);
        let index = cache.index().clone();
        let http = JottaClient::http_client(&config.network.user_agent, std::time::Duration::from_millis(config.network.timeout_ms))?;
        let store = Arc::new(nidvue_core::secret::EncryptedFileStore::new(config.credentials_file()));
        let auth = Arc::new(AuthManager::new(http.clone(), store));
        let _ = auth.restore().await?;
        let username = auth.username().await.unwrap_or_else(|| "unknown".to_string());
        let jotta = Arc::new(JottaClient::new(http, auth.clone(), JottaConfig::from_app(&config, username)));
        let (events, _) = broadcast::channel(1024);
        Ok(Self {
            inner: Arc::new(Inner {
                config,
                auth,
                jotta,
                cache,
                index,
                events,
                cancel: CancellationToken::new(),
                started: Instant::now(),
            }),
        })
    }

    /// The effective configuration.
    pub fn config(&self) -> &AppConfig {
        &self.inner.config
    }

    /// Exchange a Personal Login Token and persist the result.
    pub async fn login(&self, login_token: &str) -> Result<String> {
        self.inner.auth.login(login_token).await
    }

    /// Forget the stored token.
    pub async fn logout(&self) -> Result<()> {
        self.inner.auth.logout().await
    }

    /// True when a token is present.
    pub async fn is_logged_in(&self) -> bool {
        self.inner.auth.is_logged_in().await
    }

    /// The account username, when known.
    pub async fn username(&self) -> Option<String> {
        self.inner.auth.username().await
    }

    /// Subscribe to engine events.
    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.inner.events.subscribe()
    }

    /// A token that cancels every in-flight operation.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.inner.cancel.clone()
    }

    /// The metadata index.
    pub fn index(&self) -> &Arc<Index> {
        &self.inner.index
    }

    /// The cache.
    pub fn cache(&self) -> &Arc<Cache> {
        &self.inner.cache
    }

    /// The Jottacloud client.
    pub fn jotta(&self) -> &Arc<JottaClient> {
        &self.inner.jotta
    }

    /// A dashboard snapshot.
    pub async fn stats(&self) -> Result<StatsSnapshot> {
        let usage = self.inner.cache.usage()?;
        let conflicts = self.inner.index.conflicts(true)?.len() as u64;
        let entries = self.inner.index.count_entries()?;
        let health = if !self.is_logged_in().await || conflicts > 0 {
            HealthState::Degraded
        } else {
            HealthState::Ok
        };
        Ok(StatsSnapshot {
            upload_bps: 0,
            download_bps: 0,
            queue_depth: 0,
            active_transfers: 0,
            cache_bytes: usage.bytes,
            cache_capacity: self.inner.config.cache.max_size_bytes,
            index_entries: entries,
            conflicts_open: conflicts,
            health,
            uptime_s: self.inner.started.elapsed().as_secs(),
        })
    }

    /// `status --json` output.
    pub async fn status(&self) -> Result<Status> {
        let usage = self.inner.cache.usage()?;
        let stats = self.stats().await?;
        let quota = match self.inner.auth.is_logged_in().await {
            true => self.inner.jotta.quota().await.ok().map(QuotaView::from),
            false => None,
        };
        Ok(Status {
            version: nidvue_core::VERSION.to_string(),
            logged_in: self.inner.auth.is_logged_in().await,
            username: self.inner.auth.username().await,
            index_entries: stats.index_entries,
            conflicts_open: stats.conflicts_open,
            cache: usage.into(),
            health: stats.health,
            uptime_s: stats.uptime_s,
            quota,
        })
    }

    /// Mount the mountpoint at `mountpoint`.
    pub fn mount(&self, mountpoint: &Path, options: MountOptions) -> Result<MountHandle> {
        let vfs = EngineVfs::new(
            self.inner.jotta.clone(),
            self.inner.cache.clone(),
            self.inner.config.spool_dir(),
        )?
        .read_only(options.read_only);
        nidvue_mount::mount(Arc::new(vfs), mountpoint, options)
    }

    /// A sync engine over this client's cache, index and remote.
    pub fn sync_engine(&self) -> SyncEngine {
        SyncEngine::new(
            self.inner.jotta.clone(),
            self.inner.index.clone(),
            self.inner.config.sync.clone(),
            self.inner.config.trash_dir(),
            self.inner.events.clone(),
        )
    }

    /// Run one sync pass for every configured, unpaused pair.
    pub async fn sync_now(&self) -> Result<Vec<SyncReport>> {
        let engine = self.sync_engine();
        let mut reports = Vec::new();
        for pair in &self.inner.config.sync.pairs {
            reports.push(engine.sync_pair(pair).await?);
        }
        Ok(reports)
    }

    /// The configured sync pairs.
    pub fn sync_pairs(&self) -> &[SyncPair] {
        &self.inner.config.sync.pairs
    }

    /// Open conflicts, newest first.
    pub fn conflicts(&self) -> Result<Vec<ConflictRow>> {
        self.inner.index.conflicts(true)
    }

    /// Resolve one conflict.
    pub fn resolve_conflict(&self, id: &str, policy: ConflictPolicy) -> Result<()> {
        self.inner.index.resolve_conflict(id, policy)?;
        let _ = self.inner.events.send(EventEnvelope::new(Event::ConflictResolved {
            id: id.to_string(),
            resolution: policy.as_str().to_string(),
        }));
        Ok(())
    }

    /// Load the plugins installed below `<state>/plugins`.
    pub fn plugins(&self) -> Result<PluginHost> {
        let mut host = PluginHost::new()?;
        let dir = self.inner.config.state_dir.join("plugins");
        if dir.is_dir() {
            for (path, result) in host.load_all(&dir) {
                if let Err(e) = &result {
                    tracing::warn!(plugin = %path.display(), error = %e, "plugin not loaded");
                }
            }
        }
        Ok(host)
    }

    /// List a remote directory.
    pub async fn list(&self, path: &RemotePath) -> Result<ListStream> {
        self.inner.jotta.list_stream(path).await
    }

    /// Stat one remote path.
    pub async fn stat(&self, path: &RemotePath) -> Result<RemoteEntry> {
        self.inner.jotta.stat(path).await
    }

    /// Upload a local file.
    pub async fn upload(&self, remote: &RemotePath, local: &Path) -> Result<RemoteEntry> {
        self.inner.jotta.upload(remote, local, None).await
    }

    /// Download a remote file into a local path, atomically.
    pub async fn download(&self, remote: &RemotePath, local: &Path) -> Result<u64> {
        if let Some(parent) = local.parent() {
            std::fs::create_dir_all(parent).map_err(|e| NidvueError::io(parent, e))?;
        }
        let tmp = local.with_extension("nidvue-tmp");
        let mut file = tokio::fs::File::create(&tmp).await.map_err(|e| NidvueError::io(&tmp, e))?;
        let size = self.inner.jotta.download_to(remote, &mut file, None, None).await?;
        file.sync_all().await.map_err(|e| NidvueError::io(&tmp, e))?;
        drop(file);
        tokio::fs::rename(&tmp, local).await.map_err(|e| NidvueError::io(local, e))?;
        Ok(size)
    }

    /// The JSON schema for every event and configuration type.
    pub fn json_schema() -> String {
        #[derive(schemars::JsonSchema)]
        #[allow(dead_code)]
        struct Schema {
            config: AppConfig,
            event: Event,
            envelope: EventEnvelope,
            status: Status,
        }
        let schema = schemars::schema_for!(Schema);
        serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn opens_a_state_directory_and_reports_status() {
        let dir = tempfile::tempdir().unwrap();
        let client = Client::open(dir.path()).await.unwrap();
        assert!(!client.is_logged_in().await);
        let status = client.status().await.unwrap();
        assert_eq!(status.version, nidvue_core::VERSION);
        assert!(!status.logged_in);
        assert_eq!(status.index_entries, 0);
        assert!(status.cache.bytes == 0);
        assert!(dir.path().join("db/index.sqlite").exists());
    }

    #[tokio::test]
    async fn config_round_trips_through_the_state_directory() {
        let dir = tempfile::tempdir().unwrap();
        let client = Client::open(dir.path()).await.unwrap();
        client.config().clone().save(dir.path()).unwrap();
        let again = Client::open(dir.path()).await.unwrap();
        assert_eq!(again.config().cache.max_size_bytes, client.config().cache.max_size_bytes);
    }

    #[test]
    fn schema_covers_events_and_config() {
        let schema = Client::json_schema();
        assert!(schema.contains("\"event\""));
        assert!(schema.contains("\"config\""));
        assert!(schema.contains("transfer_progress"));
    }
}
