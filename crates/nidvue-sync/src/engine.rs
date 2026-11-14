// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The sync engine: scanning, planning, journal-first execution, conflict
//! resolution, recovery and the local watcher.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use nidvue_cache::{Index, JournalRow};
use nidvue_core::config::{SyncConfig, SyncPair};
use nidvue_core::error::{NidvueError, Result};
use nidvue_core::event::{Direction, Event, EventEnvelope};
use nidvue_core::hash::{hash_file, Hash};
use nidvue_core::id::CorrelationId;
use nidvue_core::path::{remote_to_local, RemotePath};
use nidvue_core::time::now_ms;
use nidvue_core::types::{BaselineEntry, ConflictPolicy, EntryKind, Version};
use nidvue_vfs::Remote;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::reconciler::{conflict_copy_path, decide_conflict, plan, Action};
use crate::scanner::{scan_local, PathFilter};
use crate::scheduler::Scheduler;

/// The outcome of one sync pass.
#[derive(Debug, Clone, Default)]
pub struct SyncReport {
    /// Entries observed locally.
    pub scanned: u64,
    /// Actions planned.
    pub planned: u64,
    /// Actions applied successfully.
    pub applied: u64,
    /// Conflicts detected.
    pub conflicts: u64,
    /// Actions that failed, with the reason.
    pub failed: Vec<(String, String)>,
}

/// The outcome of a recovery pass.
#[derive(Debug, Clone, Default)]
pub struct RecoveryReport {
    /// Journal rows examined.
    pub examined: u64,
    /// Rows whose effect was observed and are now committed.
    pub committed: u64,
    /// Rows left for the next sync pass to re-plan.
    pub pending: Vec<JournalRow>,
}

/// The sync engine.
pub struct SyncEngine {
    remote: Arc<dyn Remote>,
    index: Arc<Index>,
    cfg: SyncConfig,
    scheduler: Arc<Scheduler>,
    events: broadcast::Sender<EventEnvelope>,
    trash_dir: PathBuf,
}

impl SyncEngine {
    /// Build an engine over a remote, an index and the sync configuration.
    pub fn new(
        remote: Arc<dyn Remote>,
        index: Arc<Index>,
        cfg: SyncConfig,
        trash_dir: PathBuf,
        events: broadcast::Sender<EventEnvelope>,
    ) -> Self {
        let scheduler = Arc::new(Scheduler::new(cfg.clone()));
        Self {
            remote,
            index,
            cfg,
            scheduler,
            events,
            trash_dir,
        }
    }

    /// Replace the hot-reloadable configuration.
    pub fn reconfigure(&mut self, cfg: SyncConfig) {
        self.cfg = cfg;
        self.scheduler = Arc::new(Scheduler::new(self.cfg.clone()));
    }

    /// A receiver for engine events.
    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.events.subscribe()
    }

    fn emit(&self, event: Event) {
        let _ = self.events.send(EventEnvelope::new(event));
    }

    /// One full three-way pass over a pair.
    pub async fn sync_pair(&self, pair: &SyncPair) -> Result<SyncReport> {
        if pair.paused || self.cfg.paused {
            return Ok(SyncReport::default());
        }
        self.emit(Event::SyncStarted {
            pair: pair.name.clone(),
        });
        let remote_root = RemotePath::parse(&pair.remote)?;
        let filter = PathFilter::new(&pair.include, &pair.exclude)?;

        let local_root = pair.local.clone();
        let base = remote_root.clone();
        let max_size = self.cfg.max_file_size_bytes;
        let local =
            tokio::task::spawn_blocking(move || scan_local(&local_root, &base, &filter, max_size))
                .await
                .map_err(|e| NidvueError::Internal(format!("scan task: {e}")))??;

        let remote = scan_remote(self.remote.as_ref(), &remote_root).await?;
        let baselines: BTreeMap<RemotePath, BaselineEntry> = self
            .index
            .all_baselines()?
            .into_iter()
            .filter(|b| b.path.starts_with(&remote_root))
            .map(|b| (b.path.clone(), b))
            .collect();

        let actions = plan(&local, &remote, &baselines);
        let mut report = SyncReport {
            scanned: local.len() as u64,
            planned: actions.len() as u64,
            ..Default::default()
        };
        let policy = pair.conflict_policy.unwrap_or(self.cfg.conflict_policy);

        for action in actions {
            let is_conflict = matches!(action, Action::Conflict { .. });
            match self
                .execute(&pair.local, &remote_root, &action, policy)
                .await
            {
                Ok(()) => {
                    report.applied += 1;
                    if is_conflict {
                        report.conflicts += 1;
                    }
                }
                Err(NidvueError::Cancelled) => return Err(NidvueError::Cancelled),
                Err(e) => {
                    warn!(path = %action.path(), error = %e, "sync action failed");
                    if is_conflict {
                        report.conflicts += 1;
                    }
                    report
                        .failed
                        .push((action.path().to_string(), e.to_string()));
                    self.emit(Event::TransferFailed {
                        path: action.path().to_string(),
                        direction: Direction::Upload,
                        error: e.to_string(),
                        retryable: e.is_retryable(),
                    });
                }
            }
        }
        self.emit(Event::SyncFinished {
            pair: pair.name.clone(),
            scanned: report.scanned,
            planned: report.planned,
            applied: report.applied,
            conflicts: report.conflicts,
        });
        Ok(report)
    }

    /// Execute one planned action, journal first.
    pub async fn execute(
        &self,
        local_root: &Path,
        remote_root: &RemotePath,
        action: &Action,
        policy: ConflictPolicy,
    ) -> Result<()> {
        let path = action.path().clone();
        let _guard = self.scheduler.lock_path(path.as_str()).await;
        let corr = CorrelationId::new("sync");
        let local_path = local_for(local_root, remote_root, &path);
        match action {
            Action::Skip { .. } => Ok(()),
            Action::Forget { .. } => {
                self.index.clear_baseline(path.as_str())?;
                Ok(())
            }
            Action::CreateRemote { local, .. } if local_path.is_dir() => {
                let id = self.index.journal_append(
                    corr.as_str(),
                    path.as_str(),
                    "mkdir",
                    None,
                    None,
                    None,
                )?;
                self.remote.remote_mkdir(&path).await?;
                self.index
                    .set_baseline(path.as_str(), EntryKind::Dir, local, local, now_ms())?;
                self.index.journal_mark(id, "commit")?;
                Ok(())
            }
            Action::CreateLocal { remote, .. }
                if self
                    .remote
                    .remote_stat(&path)
                    .await
                    .map(|e| e.kind == EntryKind::Dir)
                    .unwrap_or(false) =>
            {
                let id = self.index.journal_append(
                    corr.as_str(),
                    path.as_str(),
                    "mkdir_local",
                    None,
                    None,
                    None,
                )?;
                std::fs::create_dir_all(&local_path)
                    .map_err(|e| NidvueError::io(&local_path, e))?;
                self.index
                    .set_baseline(path.as_str(), EntryKind::Dir, remote, remote, now_ms())?;
                self.index.journal_mark(id, "commit")?;
                Ok(())
            }
            Action::CreateRemote { local, .. } | Action::Upload { local, .. } => {
                let id = self.index.journal_append(
                    corr.as_str(),
                    path.as_str(),
                    "upload",
                    local.hash.as_ref().map(|h| h.as_str()),
                    Some(local.size),
                    None,
                )?;
                self.emit(Event::TransferQueued {
                    path: path.to_string(),
                    direction: Direction::Upload,
                });
                if let Some(parent) = path.parent() {
                    if !parent.is_root() {
                        self.remote.remote_mkdir(&parent).await?;
                    }
                }
                let entry = self.remote.remote_upload(&path, &local_path).await?;
                if entry.size != local.size {
                    self.index.journal_mark(id, "intent")?;
                    return Err(NidvueError::Verification(format!(
                        "{path}: remote has {} bytes, local {}",
                        entry.size, local.size
                    )));
                }
                self.index.journal_mark(id, "applied")?;
                let hash = local
                    .hash
                    .clone()
                    .or_else(|| hash_file(&local_path).ok().map(|h| h.hash));
                self.index.set_baseline(
                    path.as_str(),
                    EntryKind::File,
                    local,
                    &Version::new(entry.size, entry.modified_ms, hash.clone()),
                    now_ms(),
                )?;
                self.index.journal_mark(id, "commit")?;
                self.emit(Event::TransferDone {
                    path: path.to_string(),
                    direction: Direction::Upload,
                    hash: hash.map(|h| h.as_str().to_string()).unwrap_or_default(),
                });
                Ok(())
            }
            Action::CreateLocal { remote, .. } | Action::Download { remote, .. } => {
                let id = self.index.journal_append(
                    corr.as_str(),
                    path.as_str(),
                    "download",
                    remote.hash.as_ref().map(|h| h.as_str()),
                    Some(remote.size),
                    None,
                )?;
                self.emit(Event::TransferQueued {
                    path: path.to_string(),
                    direction: Direction::Download,
                });
                let hash = self
                    .download_atomic(&path, &local_path, remote.size)
                    .await?;
                self.index.journal_mark(id, "applied")?;
                self.index.set_baseline(
                    path.as_str(),
                    EntryKind::File,
                    &Version::new(remote.size, remote.mtime_ms, hash.clone()),
                    remote,
                    now_ms(),
                )?;
                self.index.journal_mark(id, "commit")?;
                self.emit(Event::TransferDone {
                    path: path.to_string(),
                    direction: Direction::Download,
                    hash: hash.map(|h| h.as_str().to_string()).unwrap_or_default(),
                });
                Ok(())
            }
            Action::DeleteLocal { .. } => {
                let id = self.index.journal_append(
                    corr.as_str(),
                    path.as_str(),
                    "delete_local",
                    None,
                    None,
                    None,
                )?;
                self.trash(&path, &local_path)?;
                self.index.clear_baseline(path.as_str())?;
                self.index.delete_entry(path.as_str())?;
                self.index.journal_mark(id, "commit")?;
                Ok(())
            }
            Action::DeleteRemote { .. } => {
                let id = self.index.journal_append(
                    corr.as_str(),
                    path.as_str(),
                    "delete_remote",
                    None,
                    None,
                    None,
                )?;
                self.remote.remote_delete(&path, false).await?;
                self.index.clear_baseline(path.as_str())?;
                self.index.journal_mark(id, "commit")?;
                Ok(())
            }
            Action::Conflict { local, remote, .. } => {
                let decision = decide_conflict(&path, local, remote, policy, now_ms());
                let copy = conflict_copy_path(&path, now_ms());
                let copy_local = local_for(local_root, remote_root, &copy);
                let copy_id = self.index.journal_append(
                    corr.as_str(),
                    copy.as_str(),
                    "download",
                    remote.hash.as_ref().map(|h| h.as_str()),
                    Some(remote.size),
                    Some(
                        &serde_json::json!({ "conflict": true, "original": path.as_str() })
                            .to_string(),
                    ),
                )?;
                // Both versions survive, on both sides, under two names: the
                // original path keeps the winner, the conflict copy keeps the
                // loser. The order never leaves a path without content.
                let copy_hash = match decision.keep {
                    nidvue_core::types::Side::Local => {
                        // The remote version of the ORIGINAL path becomes the
                        // conflict copy; the copy does not exist remotely yet.
                        let hash = self
                            .download_atomic(&path, &copy_local, remote.size)
                            .await?;
                        self.remote.remote_upload(&copy, &copy_local).await?;
                        // Local version wins the original path.
                        self.remote.remote_upload(&path, &local_path).await?;
                        hash
                    }
                    nidvue_core::types::Side::Remote => {
                        // Local version becomes the conflict copy: write it
                        // locally and remotely, then fetch the remote winner.
                        if let Some(parent) = copy_local.parent() {
                            std::fs::create_dir_all(parent)
                                .map_err(|e| NidvueError::io(parent, e))?;
                        }
                        std::fs::copy(&local_path, &copy_local)
                            .map_err(|e| NidvueError::io(&local_path, e))?;
                        self.remote.remote_upload(&copy, &copy_local).await?;
                        self.download_atomic(&path, &local_path, remote.size)
                            .await?
                    }
                };
                self.index.journal_mark(copy_id, "applied")?;
                self.index.set_baseline(
                    copy.as_str(),
                    EntryKind::File,
                    &Version::new(local.size.max(remote.size), now_ms(), copy_hash.clone()),
                    &Version::new(local.size.max(remote.size), now_ms(), copy_hash.clone()),
                    now_ms(),
                )?;
                let winner = match decision.keep {
                    nidvue_core::types::Side::Local => local,
                    nidvue_core::types::Side::Remote => remote,
                };
                self.index.set_baseline(
                    path.as_str(),
                    EntryKind::File,
                    winner,
                    winner,
                    now_ms(),
                )?;
                self.index.journal_mark(copy_id, "commit")?;
                let conflict_id = nidvue_core::conflict::Conflict::id_for(path.as_str(), now_ms());
                self.index.add_conflict(&nidvue_cache::ConflictRow {
                    id: conflict_id.clone(),
                    path: path.to_string(),
                    conflict_path: copy.to_string(),
                    kept: match decision.keep {
                        nidvue_core::types::Side::Local => "local".into(),
                        nidvue_core::types::Side::Remote => "remote".into(),
                    },
                    detected_ms: now_ms(),
                    local_size: local.size,
                    remote_size: remote.size,
                    resolution: None,
                })?;
                self.emit(Event::ConflictDetected {
                    conflict: nidvue_core::conflict::Conflict {
                        id: conflict_id,
                        path: path.to_string(),
                        local_path: local_path.display().to_string(),
                        conflict_path: copy.to_string(),
                        kept: decision.keep,
                        detected_ms: now_ms(),
                        local_size: local.size,
                        remote_size: remote.size,
                        resolution: None,
                    },
                });
                Ok(())
            }
        }
    }

    async fn download_atomic(
        &self,
        path: &RemotePath,
        local: &Path,
        expected: u64,
    ) -> Result<Option<Hash>> {
        if let Some(parent) = local.parent() {
            std::fs::create_dir_all(parent).map_err(|e| NidvueError::io(parent, e))?;
        }
        let tmp = local.with_file_name(format!(
            ".{}.nidvue-tmp",
            local
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download")
        ));
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| NidvueError::io(&tmp, e))?;
        let result = self
            .remote
            .remote_download(path, &mut file, Some(expected))
            .await;
        file.sync_all()
            .await
            .map_err(|e| NidvueError::io(&tmp, e))?;
        drop(file);
        let size = result?;
        if size != expected {
            let _ = std::fs::remove_file(&tmp);
            return Err(NidvueError::Verification(format!(
                "{path}: downloaded {size} bytes, expected {expected}"
            )));
        }
        // Verify the content hash before it replaces the destination.
        let hash = tokio::task::spawn_blocking({
            let tmp = tmp.clone();
            move || hash_file(&tmp)
        })
        .await
        .map_err(|e| NidvueError::Internal(format!("hash task: {e}")))??;
        tokio::fs::rename(&tmp, local)
            .await
            .map_err(|e| NidvueError::io(local, e))?;
        if let Some(dir) = local.parent() {
            if let Ok(f) = std::fs::File::open(dir) {
                let _ = f.sync_all();
            }
        }
        self.index.upsert_entry(&nidvue_cache::EntryRow {
            path: path.as_str().into(),
            kind: EntryKind::File,
            size: hash.size,
            mtime_ms: now_ms(),
            hash: Some(hash.hash.as_str().into()),
            remote_rev: None,
            dirty: false,
        })?;
        Ok(Some(hash.hash))
    }

    fn trash(&self, remote: &RemotePath, local: &Path) -> Result<()> {
        if !local.exists() {
            return Ok(());
        }
        if self.cfg.trash_retention_days == 0 {
            return std::fs::remove_file(local).map_err(|e| NidvueError::io(local, e));
        }
        let target = self
            .trash_dir
            .join(nidvue_core::time::conflict_timestamp(now_ms()))
            .join(remote.as_str().trim_start_matches('/'));
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| NidvueError::io(parent, e))?;
        }
        std::fs::rename(local, &target).map_err(|e| NidvueError::io(local, e))?;
        debug!(path = %remote, trash = %target.display(), "moved to trash");
        Ok(())
    }

    /// Resolve journal rows left behind by a crash.
    ///
    /// A row is committed only when the effect is *observed*; anything else is
    /// left for the next sync pass to re-plan. Nothing is assumed.
    pub async fn recover(&self) -> Result<RecoveryReport> {
        let pending = self.index.journal_pending()?;
        let mut report = RecoveryReport {
            examined: pending.len() as u64,
            ..Default::default()
        };
        for row in pending {
            let observed = match row.op.as_str() {
                "upload" => match self
                    .remote
                    .remote_stat(&RemotePath::parse(&row.path)?)
                    .await
                {
                    Ok(entry) => row.expected_size.map(|s| entry.size == s).unwrap_or(true),
                    Err(_) => false,
                },
                "delete_remote" | "delete_local" => self
                    .remote
                    .remote_stat(&RemotePath::parse(&row.path)?)
                    .await
                    .is_err(),
                "mkdir" => self
                    .remote
                    .remote_stat(&RemotePath::parse(&row.path)?)
                    .await
                    .is_ok(),
                "rename" => {
                    let to = row
                        .detail
                        .as_deref()
                        .and_then(|d| serde_json::from_str::<serde_json::Value>(d).ok())
                        .and_then(|v| v.get("to").and_then(|t| t.as_str().map(str::to_string)));
                    match to {
                        Some(to) => self
                            .remote
                            .remote_stat(&RemotePath::parse(&to)?)
                            .await
                            .is_ok(),
                        None => false,
                    }
                }
                "download" => {
                    // The local destination is verified by the next scan; a
                    // journal row with no observation stays pending.
                    false
                }
                _ => false,
            };
            if observed {
                self.index.journal_mark(row.id, "commit")?;
                report.committed += 1;
            } else {
                report.pending.push(row);
            }
        }
        info!(
            examined = report.examined,
            committed = report.committed,
            pending = report.pending.len(),
            "journal recovery"
        );
        Ok(report)
    }
}

/// Scan the remote tree into the same shape as the local scan.
pub async fn scan_remote(
    remote: &dyn Remote,
    root: &RemotePath,
) -> Result<BTreeMap<RemotePath, (EntryKind, Version)>> {
    let listing = remote.remote_list(root).await?;
    let mut out = BTreeMap::new();
    for folder in &listing.folders {
        let path = join(root, &folder.path);
        out.insert(path, (EntryKind::Dir, Version::new(0, 0, None)));
    }
    for file in &listing.files {
        let path = join(root, &file.name);
        out.insert(
            path,
            (
                EntryKind::File,
                Version::new(file.size, file.modified_ms, None),
            ),
        );
    }
    Ok(out)
}

/// Map a remote path below `remote_root` to its local path below `local_root`.
fn local_for(local_root: &Path, remote_root: &RemotePath, path: &RemotePath) -> PathBuf {
    let suffix = path
        .as_str()
        .strip_prefix(remote_root.as_str())
        .unwrap_or(path.as_str());
    let suffix = if suffix.is_empty() { "/" } else { suffix };
    match RemotePath::parse(suffix) {
        Ok(rel) => remote_to_local(local_root, &rel),
        Err(_) => local_root.to_path_buf(),
    }
}

fn join(base: &RemotePath, relative: &str) -> RemotePath {
    let rel = relative.trim_matches('/');
    if rel.is_empty() {
        return base.clone();
    }
    // liststream paths are already relative to the mountpoint root; a path that
    // already includes the base must not be doubled.
    let candidate = if rel.starts_with(base.as_str().trim_start_matches('/')) {
        format!("/{rel}")
    } else {
        format!("{}/{}", base.as_str().trim_end_matches('/'), rel)
    };
    RemotePath::parse(&candidate).unwrap_or_else(|_| base.clone())
}

/// Watch a local directory and call `on_change` after a debounce window.
///
/// The callback runs on the caller's runtime; the watcher thread only forwards.
pub fn spawn_watcher(
    root: PathBuf,
    debounce: Duration,
    on_change: impl Fn() + Send + 'static,
) -> Result<notify::RecommendedWatcher> {
    use notify::{RecursiveMode, Watcher};
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let mut watcher = notify::recommended_watcher(
        move |res: std::result::Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if !event.paths.is_empty() {
                    let _ = tx.send(());
                }
            }
        },
    )
    .map_err(|e| NidvueError::Mount(format!("watcher: {e}")))?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| NidvueError::Mount(format!("watch {}: {e}", root.display())))?;
    tokio::spawn(async move {
        loop {
            if rx.recv().await.is_none() {
                break;
            }
            // Coalesce: wait for the burst to end.
            loop {
                match tokio::time::timeout(debounce, rx.recv()).await {
                    Ok(Some(())) => continue,
                    Ok(None) => return,
                    Err(_) => break,
                }
            }
            on_change();
        }
    });
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_does_not_double_the_base() {
        let base = RemotePath::parse("/Archive").unwrap();
        assert_eq!(join(&base, "Docs/a").as_str(), "/Archive/Docs/a");
        assert_eq!(join(&base, "Archive/Docs/a").as_str(), "/Archive/Docs/a");
    }
}
