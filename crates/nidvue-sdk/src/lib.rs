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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use nidvue_cache::{Cache, CacheUsage, ConflictRow, Index};
use nidvue_core::config::{AppConfig, SyncPair};
use nidvue_core::error::{NidvueError, Result};
use nidvue_core::event::{Event, EventEnvelope, HealthState, StatsSnapshot};
use nidvue_jotta::auth::AuthManager;
use nidvue_jotta::client::{JottaClient, JottaConfig, Quota};
use nidvue_mount::{MountHandle, MountOptions};
use nidvue_plugin::PluginHost;
use nidvue_sync::{SyncEngine, SyncReport};
use nidvue_telemetry::{MetricsCache, OtlpExporter, PrometheusServer};
use nidvue_vfs::EngineVfs;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub use nidvue_core::config::{CacheConfig, LogConfig, MountConfig, NetworkConfig, SyncConfig};
pub use nidvue_core::conflict::{Conflict, Resolution};
pub use nidvue_core::error::NidvueError as Error;
pub use nidvue_core::metrics::{MetricSample, MetricsSnapshot};
pub use nidvue_core::path::RemotePath;
pub use nidvue_core::transfers::{Transfer, TransferRegistry, TransferState};
pub use nidvue_core::types::{ConflictPolicy, EntryKind, EntryMeta, Version};
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
    /// True when the engine is paused.
    pub paused: bool,
    /// Aggregate bandwidth cap in bytes/second (0 = unlimited).
    pub bandwidth_limit_bps: u64,
    /// Cache counters.
    pub cache: CacheUsageView,
    /// Overall health.
    pub health: HealthState,
    /// Seconds since the client was opened.
    pub uptime_s: u64,
    /// Account quota, when the server answered.
    pub quota: Option<QuotaView>,
    /// Transfers the engine has announced, newest last, bounded.
    pub transfers: Vec<Transfer>,
    /// How many rows the transfer list keeps.
    pub transfer_limit: usize,
    /// Where credentials live, and why when it is a fallback.
    pub secret_backend: String,
    /// True when the requested credential store cannot work.
    pub secret_degraded: bool,
    /// The scrape URL, when a Prometheus endpoint is running.
    pub prometheus_url: Option<String>,
    /// The collector URL, when an OTLP exporter is running.
    pub otlp_endpoint: Option<String>,
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
    /// Configured capacity in bytes.
    pub capacity: u64,
}

impl From<CacheUsage> for CacheUsageView {
    fn from(u: CacheUsage) -> Self {
        Self {
            bytes: u.bytes,
            blobs: u.blobs,
            pinned: u.pinned,
            dirty: u.dirty,
            capacity: 0,
        }
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
        Self {
            usage: q.usage,
            quota: q.quota,
        }
    }
}

struct Inner {
    config: RwLock<AppConfig>,
    auth: Arc<AuthManager>,
    jotta: Arc<JottaClient>,
    cache: Arc<Cache>,
    index: Arc<Index>,
    events: broadcast::Sender<EventEnvelope>,
    cancel: CancellationToken,
    started: Instant,
    paused: AtomicBool,
    bandwidth_bps: AtomicU64,
    transfers: TransferRegistry,
    metrics: MetricsCache,
    secret_backend: String,
    secret_degraded: bool,
    prometheus: Option<PrometheusServer>,
    otlp_endpoint: Option<String>,
}

/// The Nidvue Drive client.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

fn start_prometheus(config: &AppConfig, metrics: &MetricsCache) -> Option<PrometheusServer> {
    let bind = config.log.prometheus_bind.as_deref()?.trim();
    if bind.is_empty() {
        return None;
    }
    match PrometheusServer::start(bind, metrics.clone()) {
        Ok(server) => Some(server),
        Err(e) => {
            tracing::warn!(%bind, error = %e, "prometheus endpoint not started");
            None
        }
    }
}

fn start_otlp(config: &AppConfig, metrics: &MetricsCache) -> Option<String> {
    let endpoint = config.log.otel_endpoint.as_deref()?.trim();
    if endpoint.is_empty() {
        return None;
    }
    match OtlpExporter::new(endpoint, metrics.clone()) {
        Ok(exporter) => {
            let every = Duration::from_secs(config.log.otel_interval_s.max(1));
            exporter.spawn(every);
            Some(endpoint.to_string())
        }
        Err(e) => {
            tracing::warn!(%endpoint, error = %e, "otlp exporter not started");
            None
        }
    }
}

impl Client {
    /// Open (or create) a client state directory.
    pub async fn open(state_dir: impl Into<PathBuf>) -> Result<Self> {
        let state_dir = state_dir.into();
        let config = AppConfig::load_or_default(&state_dir)?;
        std::fs::create_dir_all(config.db_dir())
            .map_err(|e| NidvueError::io(config.db_dir(), e))?;
        let cache = Arc::new(Cache::open(
            &config.cache,
            &config.blobs_dir(),
            &config.db_dir().join("index.sqlite"),
        )?);
        let index = cache.index().clone();
        let http = JottaClient::http_client(
            &config.network.user_agent,
            std::time::Duration::from_millis(config.network.timeout_ms),
        )?;
        let selection =
            nidvue_core::secret::open_store(config.secret.backend, config.credentials_file());
        let secret_backend = selection.describe();
        let secret_degraded = selection.degraded;
        let auth = Arc::new(AuthManager::new(http.clone(), selection.store));
        let _ = auth.restore().await?;
        let username = auth
            .username()
            .await
            .unwrap_or_else(|| "unknown".to_string());
        let jotta = Arc::new(JottaClient::new(
            http,
            auth.clone(),
            JottaConfig::from_app(&config, username),
        ));
        let (events, _) = broadcast::channel(1024);
        let paused = AtomicBool::new(config.sync.paused);
        let bandwidth_bps = AtomicU64::new(config.sync.bandwidth_limit_bps);
        let transfers = TransferRegistry::new();
        let metrics = MetricsCache::new();
        let prometheus = start_prometheus(&config, &metrics);
        let otlp_endpoint = start_otlp(&config, &metrics);
        let client = Self {
            inner: Arc::new(Inner {
                paused,
                bandwidth_bps,
                config: RwLock::new(config),
                auth,
                jotta,
                cache,
                index,
                events,
                cancel: CancellationToken::new(),
                started: Instant::now(),
                transfers,
                metrics,
                secret_backend,
                secret_degraded,
                prometheus,
                otlp_endpoint,
            }),
        };
        client.spawn_feeders();
        client.spawn_span_export();
        Ok(client)
    }

    /// Post one OTLP/HTTP JSON span per finished pass, failed transfer and
    /// error transition. The metrics exporter in `nidvue-telemetry` shares the
    /// endpoint and posts to `/v1/metrics`; this one posts to `/v1/traces`.
    fn spawn_span_export(&self) {
        let Some(endpoint) = self.read_config().log.otel_endpoint.clone() else {
            return;
        };
        if endpoint.trim().is_empty() {
            return;
        }
        let exporter = match nidvue_jotta::otlp::OtlpExporter::new(&endpoint) {
            Ok(exporter) => exporter,
            Err(e) => {
                tracing::warn!(%endpoint, error = %e, "otlp span exporter not started");
                return;
            }
        };
        let mut events = self.inner.events.subscribe();
        tokio::spawn(async move {
            while let Ok(envelope) = events.recv().await {
                let spans = nidvue_jotta::otlp::spans_from_event(&envelope);
                if spans.is_empty() {
                    continue;
                }
                if let Err(e) = exporter.send(&spans).await {
                    tracing::warn!(error = %e, "otlp span export failed");
                }
            }
        });
    }

    fn spawn_feeders(&self) {
        let mut events = self.inner.events.subscribe();
        let transfers = self.inner.transfers.clone();
        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(envelope) => transfers.record(&envelope.event),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        let refresher = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(2));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if let Ok(snapshot) = refresher.metrics_snapshot().await {
                    refresher.inner.metrics.store(snapshot);
                }
            }
        });
    }

    /// The metric snapshot both exporters publish.
    pub async fn metrics_snapshot(&self) -> Result<MetricsSnapshot> {
        let stats = self.stats().await?;
        let status = self.status().await?;
        let mut samples = vec![
            MetricSample::new(
                "nidvue_cache_bytes",
                "Bytes stored in the blob cache.",
                stats.cache_bytes as f64,
            ),
            MetricSample::new(
                "nidvue_cache_capacity_bytes",
                "Configured cache capacity in bytes.",
                stats.cache_capacity as f64,
            ),
            MetricSample::new(
                "nidvue_cache_blobs",
                "Blobs in the cache.",
                status.cache.blobs as f64,
            ),
            MetricSample::new(
                "nidvue_cache_pinned",
                "Pinned blobs in the cache.",
                status.cache.pinned as f64,
            ),
            MetricSample::new(
                "nidvue_cache_dirty",
                "Blobs not yet drained to the remote.",
                status.cache.dirty as f64,
            ),
            MetricSample::new(
                "nidvue_index_entries",
                "Entries in the metadata index.",
                stats.index_entries as f64,
            ),
            MetricSample::new(
                "nidvue_conflicts_open",
                "Conflicts awaiting a decision.",
                stats.conflicts_open as f64,
            ),
            MetricSample::new(
                "nidvue_queue_depth",
                "Transfers waiting to run.",
                stats.queue_depth as f64,
            ),
            MetricSample::new(
                "nidvue_active_transfers",
                "Transfers running now.",
                stats.active_transfers as f64,
            ),
            MetricSample::new(
                "nidvue_uptime_seconds",
                "Seconds since the client opened.",
                stats.uptime_s as f64,
            ),
            MetricSample::new(
                "nidvue_logged_in",
                "1 when a token is present.",
                if status.logged_in { 1.0 } else { 0.0 },
            ),
            MetricSample::new(
                "nidvue_paused",
                "1 when the engine is paused.",
                if status.paused { 1.0 } else { 0.0 },
            ),
            MetricSample::new(
                "nidvue_health",
                "1 when degraded, 0 when ok.",
                if stats.health == HealthState::Ok {
                    0.0
                } else {
                    1.0
                },
            ),
        ];
        if let Some(quota) = status.quota {
            samples.push(MetricSample::new(
                "nidvue_quota_used_bytes",
                "Account bytes used.",
                quota.usage as f64,
            ));
            samples.push(MetricSample::new(
                "nidvue_quota_limit_bytes",
                "Account quota in bytes.",
                quota.quota as f64,
            ));
        }
        Ok(MetricsSnapshot {
            service: "nidvue-drive".into(),
            version: nidvue_core::VERSION.into(),
            account: status.username,
            samples,
        })
    }

    /// The live transfer list, oldest first.
    pub fn transfers(&self) -> Vec<Transfer> {
        self.inner.transfers.list()
    }

    /// Publish an event on the bus every subscriber and the transfer list read.
    ///
    /// This is the seam a plugin uses to surface its own progress, and the one
    /// the engine itself uses.
    pub fn publish(&self, event: Event) {
        let _ = self.inner.events.send(EventEnvelope::new(event));
    }

    /// Drop every recorded transfer row.
    pub fn clear_transfers(&self) {
        self.inner.transfers.clear();
    }

    /// The effective configuration.
    pub fn config(&self) -> AppConfig {
        self.read_config().clone()
    }

    /// Read one value by dotted key, e.g. `sync.pairs.docs.exclude`.
    pub fn config_get(&self, key: &str) -> Result<serde_json::Value> {
        nidvue_core::config::dot::get(&self.read_config(), key)
    }

    /// Write one value by dotted key, persist it and apply the hot subset.
    ///
    /// The value is parsed as JSON when it parses, so a list is
    /// `["**/node_modules"]` and a bool is `true`. Everything the running
    /// engine reads live (pause, bandwidth cap, sync pairs) is applied at
    /// once; the cache cap takes effect on the next start.
    pub fn config_set(&self, key: &str, value: &str) -> Result<serde_json::Value> {
        let next = {
            let mut guard = self
                .inner
                .config
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let next = nidvue_core::config::dot::set(&guard, key, value)?;
            let dir = next.state_dir.clone();
            next.save(&dir)?;
            *guard = next.clone();
            next
        };
        self.apply_live(&next);
        nidvue_core::config::dot::get(&next, key)
    }

    fn read_config(&self) -> std::sync::RwLockReadGuard<'_, AppConfig> {
        self.inner
            .config
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Push the values the engine reads from atomics or the pair list.
    fn apply_live(&self, config: &AppConfig) {
        self.inner
            .paused
            .store(config.sync.paused, Ordering::Relaxed);
        self.inner
            .bandwidth_bps
            .store(config.sync.bandwidth_limit_bps, Ordering::Relaxed);
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

    /// True when the engine is paused.
    pub fn paused(&self) -> bool {
        self.inner.paused.load(Ordering::Relaxed)
    }

    /// Pause or resume every sync pair. Persisted in `config.toml`.
    pub fn set_paused(&self, paused: bool) -> Result<()> {
        self.config_set("sync.paused", if paused { "true" } else { "false" })?;
        let _ = self.inner.events.send(EventEnvelope::new(Event::Health {
            state: if paused {
                HealthState::Degraded
            } else {
                HealthState::Ok
            },
            detail: Some(if paused {
                "paused by the user".into()
            } else {
                "running".into()
            }),
        }));
        Ok(())
    }

    /// The aggregate bandwidth cap in bytes/second (0 = unlimited).
    pub fn bandwidth_limit_bps(&self) -> u64 {
        self.inner.bandwidth_bps.load(Ordering::Relaxed)
    }

    /// Set the aggregate bandwidth cap. Persisted in `config.toml`.
    pub fn set_bandwidth_limit_bps(&self, bytes_per_second: u64) -> Result<()> {
        self.config_set("sync.bandwidth_limit_bps", &bytes_per_second.to_string())?;
        Ok(())
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
            queue_depth: self.inner.transfers.queued(),
            active_transfers: self.inner.transfers.running(),
            cache_bytes: usage.bytes,
            cache_capacity: self.read_config().cache.max_size_bytes,
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
            paused: self.paused(),
            bandwidth_limit_bps: self.bandwidth_limit_bps(),
            cache: CacheUsageView {
                capacity: self.read_config().cache.max_size_bytes,
                ..usage.into()
            },
            health: stats.health,
            uptime_s: stats.uptime_s,
            quota,
            transfers: self.inner.transfers.list(),
            transfer_limit: nidvue_core::transfers::MAX_TRANSFERS,
            secret_backend: self.inner.secret_backend.clone(),
            secret_degraded: self.inner.secret_degraded,
            prometheus_url: self.inner.prometheus.as_ref().map(|p| p.url()),
            otlp_endpoint: self.inner.otlp_endpoint.clone(),
        })
    }

    /// Mount the mountpoint at `mountpoint`.
    pub fn mount(&self, mountpoint: &Path, options: MountOptions) -> Result<MountHandle> {
        let vfs = EngineVfs::new(
            self.inner.jotta.clone(),
            self.inner.cache.clone(),
            self.read_config().spool_dir(),
        )?
        .read_only(options.read_only);
        nidvue_mount::mount(Arc::new(vfs), mountpoint, options)
    }

    /// A sync engine over this client's cache, index and remote.
    pub fn sync_engine(&self) -> SyncEngine {
        let mut sync = self.read_config().sync.clone();
        sync.paused = self.paused();
        sync.bandwidth_limit_bps = self.bandwidth_limit_bps();
        SyncEngine::new(
            self.inner.jotta.clone(),
            self.inner.index.clone(),
            sync,
            self.read_config().trash_dir(),
            self.inner.events.clone(),
        )
    }

    /// Run one sync pass for every configured, unpaused pair.
    pub async fn sync_now(&self) -> Result<Vec<SyncReport>> {
        let engine = self.sync_engine();
        let mut reports = Vec::new();
        for pair in self.sync_pairs() {
            reports.push(engine.sync_pair(&pair).await?);
        }
        Ok(reports)
    }

    /// Run one sync pass for a single pair.
    pub async fn sync_pair_now(&self, name: &str) -> Result<SyncReport> {
        let pair = self
            .sync_pairs()
            .into_iter()
            .find(|p| p.name == name)
            .ok_or_else(|| NidvueError::Config(format!("no sync pair named {name}")))?;
        self.sync_engine().sync_pair(&pair).await
    }

    /// The configured sync pairs.
    pub fn sync_pairs(&self) -> Vec<SyncPair> {
        self.read_config().sync.pairs.clone()
    }

    /// Add a sync pair and persist it. The name must be new and the remote path
    /// absolute; the local directory is created when it does not exist.
    pub fn add_sync_pair(&self, pair: SyncPair) -> Result<()> {
        if pair.name.trim().is_empty() {
            return Err(NidvueError::Config(
                "sync pair name must not be empty".into(),
            ));
        }
        RemotePath::parse(&pair.remote)?;
        std::fs::create_dir_all(&pair.local).map_err(|e| NidvueError::io(&pair.local, e))?;
        self.update_pairs(|pairs| {
            if pairs.iter().any(|p| p.name == pair.name) {
                return Err(NidvueError::Config(format!(
                    "sync pair {} already exists",
                    pair.name
                )));
            }
            pairs.push(pair);
            Ok(())
        })
    }

    /// Remove a sync pair by name and persist the rest.
    pub fn remove_sync_pair(&self, name: &str) -> Result<()> {
        self.update_pairs(|pairs| {
            let before = pairs.len();
            pairs.retain(|p| p.name != name);
            if pairs.len() == before {
                return Err(NidvueError::Config(format!("no sync pair named {name}")));
            }
            Ok(())
        })
    }

    /// Pause or resume one sync pair. Persisted in `config.toml`.
    pub fn set_sync_pair_paused(&self, name: &str, paused: bool) -> Result<()> {
        self.update_pairs(|pairs| {
            let pair = pairs
                .iter_mut()
                .find(|p| p.name == name)
                .ok_or_else(|| NidvueError::Config(format!("no sync pair named {name}")))?;
            pair.paused = paused;
            Ok(())
        })
    }

    /// Replace one pair's include and exclude globs. Persisted in
    /// `config.toml`; the next pass scans with the new filters.
    pub fn set_sync_pair_filters(
        &self,
        name: &str,
        include: Vec<String>,
        exclude: Vec<String>,
    ) -> Result<()> {
        self.update_pairs(|pairs| {
            let pair = pairs
                .iter_mut()
                .find(|p| p.name == name)
                .ok_or_else(|| NidvueError::Config(format!("no sync pair named {name}")))?;
            pair.include = include;
            pair.exclude = exclude;
            Ok(())
        })
    }

    fn update_pairs(
        &self,
        mutate: impl FnOnce(&mut Vec<SyncPair>) -> Result<()>,
    ) -> Result<()> {
        let mut guard = self
            .inner
            .config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut next = guard.clone();
        mutate(&mut next.sync.pairs)?;
        next.validate()?;
        let dir = next.state_dir.clone();
        next.save(&dir)?;
        *guard = next;
        Ok(())
    }

    /// Open conflicts, newest first.
    pub fn conflicts(&self) -> Result<Vec<ConflictRow>> {
        self.inner.index.conflicts(true)
    }

    /// Resolve one conflict.
    pub fn resolve_conflict(&self, id: &str, policy: ConflictPolicy) -> Result<()> {
        self.inner.index.resolve_conflict(id, policy)?;
        let _ = self
            .inner
            .events
            .send(EventEnvelope::new(Event::ConflictResolved {
                id: id.to_string(),
                resolution: policy.as_str().to_string(),
            }));
        Ok(())
    }

    /// Load the plugins installed below `<state>/plugins`.
    pub fn plugins(&self) -> Result<PluginHost> {
        let mut host = PluginHost::new()?;
        let dir = self.read_config().state_dir.join("plugins");
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
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| NidvueError::io(&tmp, e))?;
        let size = self
            .inner
            .jotta
            .download_to(remote, &mut file, None, None)
            .await?;
        file.sync_all()
            .await
            .map_err(|e| NidvueError::io(&tmp, e))?;
        drop(file);
        tokio::fs::rename(&tmp, local)
            .await
            .map_err(|e| NidvueError::io(local, e))?;
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
        assert_eq!(
            again.config().cache.max_size_bytes,
            client.config().cache.max_size_bytes
        );
    }

    #[tokio::test]
    async fn config_set_persists_and_applies_live_values() {
        let dir = tempfile::tempdir().unwrap();
        let client = Client::open(dir.path()).await.unwrap();
        client
            .add_sync_pair(SyncPair {
                name: "docs".into(),
                local: dir.path().join("docs"),
                remote: "/Docs".into(),
                paused: false,
                include: vec![],
                exclude: vec![],
                conflict_policy: None,
            })
            .unwrap();

        let value = client
            .config_set("sync.pairs.docs.exclude", r#"["**/node_modules","*.tmp"]"#)
            .unwrap();
        assert_eq!(value, serde_json::json!(["**/node_modules", "*.tmp"]));
        assert_eq!(client.sync_pairs()[0].exclude.len(), 2);

        client.config_set("sync.paused", "true").unwrap();
        assert!(client.paused());
        client.config_set("sync.bandwidth_limit_bps", "125000").unwrap();
        assert_eq!(client.bandwidth_limit_bps(), 125_000);
        assert_eq!(
            client.config_get("sync.pairs.docs.remote").unwrap(),
            serde_json::json!("/Docs")
        );

        let again = Client::open(dir.path()).await.unwrap();
        assert_eq!(
            again.sync_pairs()[0].exclude,
            vec!["**/node_modules", "*.tmp"]
        );
        assert!(again.paused());

        assert!(client
            .config_set("sync.pairs.docs.exclude", r#"["["]"#)
            .is_err());
        assert!(client.config_set("nope", "1").is_err());
        assert_eq!(
            client.sync_pairs()[0].exclude.len(),
            2,
            "a rejected value changes nothing"
        );
    }

    #[tokio::test]
    async fn pair_filters_are_validated_and_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let client = Client::open(dir.path()).await.unwrap();
        client
            .add_sync_pair(SyncPair {
                name: "docs".into(),
                local: dir.path().join("docs"),
                remote: "/Docs".into(),
                paused: false,
                include: vec![],
                exclude: vec![],
                conflict_policy: None,
            })
            .unwrap();
        client
            .set_sync_pair_filters(
                "docs",
                vec!["**/*.md".into()],
                vec!["**/node_modules".into(), "*.tmp".into()],
            )
            .unwrap();
        assert_eq!(client.sync_pairs()[0].exclude.len(), 2);
        let again = Client::open(dir.path()).await.unwrap();
        assert_eq!(again.sync_pairs()[0].include, vec!["**/*.md"]);
        assert_eq!(again.sync_pairs()[0].exclude.len(), 2);
        assert!(client
            .set_sync_pair_filters("docs", vec![], vec!["[".into()])
            .is_err());
        assert!(client
            .set_sync_pair_filters("ghost", vec![], vec![])
            .is_err());
    }

    #[test]
    fn schema_covers_events_and_config() {
        let schema = Client::json_schema();
        assert!(schema.contains("\"event\""));
        assert!(schema.contains("\"config\""));
        assert!(schema.contains("transfer_progress"));
    }
}
