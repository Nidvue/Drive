// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The virtual filesystem the mount adapters and the sync engine share.
//!
//! One async trait, [`Vfs`], with two implementations:
//!
//! - [`EngineVfs`]: read-through reads from the blob cache, write-back writes
//!   through a durable spool, metadata from the SQLite index, bytes from
//!   Jottacloud.
//! - [`MemoryVfs`]: an in-memory implementation used by tests and by the mount
//!   contract suite, so semantics are tested without a kernel.
//!
//! Paths are [`RemotePath`]s: absolute, normalised, `/`-separated.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use nidvue_cache::{Cache, EntryRow, Index};
use nidvue_core::error::{NidvueError, Result};
use nidvue_core::hash::{hash_file, Hash};
use nidvue_core::id::CorrelationId;
use nidvue_core::path::RemotePath;
use nidvue_core::time::now_ms;
use nidvue_core::types::{EntryKind, EntryMeta, Version};
pub use nidvue_jotta::client::ProgressFn;
use nidvue_jotta::xml::{ListStream, RemoteEntry};

use nidvue_jotta::JottaClient;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Filesystem-wide counters for `statfs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FsStats {
    /// Total bytes available to the account.
    pub total_bytes: u64,
    /// Bytes in use on the account.
    pub used_bytes: u64,
    /// Bytes free.
    pub free_bytes: u64,
}

/// An open file.
#[derive(Debug, Clone)]
pub struct Handle {
    /// Unique handle id.
    pub id: u64,
    /// The path it was opened for.
    pub path: RemotePath,
    /// True when the handle may write.
    pub writable: bool,
    /// Blob hash at open time, when the file was cached.
    pub hash: Option<Hash>,
}

/// The operations the mount adapters translate to.
///
/// Errors map to `errno` in the adapter; the trait never panics and never
/// blocks the caller for longer than the configured timeouts.
#[async_trait]
pub trait Vfs: Send + Sync {
    /// Metadata for one path.
    async fn getattr(&self, path: &RemotePath) -> Result<EntryMeta>;

    /// One page of a directory listing, sorted by name.
    async fn readdir(
        &self,
        path: &RemotePath,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<EntryMeta>>;

    /// Read `size` bytes at `offset`. A short read means end of file.
    async fn read(&self, path: &RemotePath, offset: u64, size: usize) -> Result<Vec<u8>>;

    /// Write bytes at `offset`, extending the file as needed.
    async fn write(&self, path: &RemotePath, offset: u64, data: &[u8]) -> Result<u64>;

    /// Create a file or directory.
    async fn create(&self, path: &RemotePath, kind: EntryKind) -> Result<Handle>;

    /// Remove a file or an empty directory.
    async fn unlink(&self, path: &RemotePath) -> Result<()>;

    /// Rename or move one path.
    async fn rename(&self, from: &RemotePath, to: &RemotePath) -> Result<()>;

    /// Set the length of a file.
    async fn truncate(&self, path: &RemotePath, size: u64) -> Result<()>;

    /// Filesystem counters.
    async fn statfs(&self) -> Result<FsStats>;

    /// Flush a dirty path to the spool, ready for the drainer.
    async fn flush(&self, path: &RemotePath) -> Result<()>;
}

/// The remote operations the engine needs, abstracted so tests can fake them.
#[async_trait]
pub trait Remote: Send + Sync {
    /// Stat one path.
    async fn remote_stat(&self, path: &RemotePath) -> Result<RemoteEntry>;
    /// List a tree.
    async fn remote_list(&self, path: &RemotePath) -> Result<ListStream>;
    /// Stream a file into a writer, reporting `(done, total)` as it goes.
    async fn remote_download(
        &self,
        path: &RemotePath,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        expected_size: Option<u64>,
        progress: Option<ProgressFn>,
    ) -> Result<u64>;
    /// Upload a local file and return its verified size.
    async fn remote_upload(
        &self,
        path: &RemotePath,
        local: &Path,
        progress: Option<ProgressFn>,
    ) -> Result<RemoteEntry>;
    /// Create a directory.
    async fn remote_mkdir(&self, path: &RemotePath) -> Result<()>;
    /// Delete a path.
    async fn remote_delete(&self, path: &RemotePath, permanent: bool) -> Result<()>;
    /// Move a path.
    async fn remote_rename(&self, from: &RemotePath, to: &RemotePath) -> Result<()>;
}

#[async_trait]
impl Remote for JottaClient {
    async fn remote_stat(&self, path: &RemotePath) -> Result<RemoteEntry> {
        self.stat(path).await
    }

    async fn remote_list(&self, path: &RemotePath) -> Result<ListStream> {
        self.list_stream(path).await
    }

    async fn remote_download(
        &self,
        path: &RemotePath,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        expected_size: Option<u64>,
        progress: Option<ProgressFn>,
    ) -> Result<u64> {
        self.download_to(path, writer, expected_size, progress)
            .await
    }

    async fn remote_upload(
        &self,
        path: &RemotePath,
        local: &Path,
        progress: Option<ProgressFn>,
    ) -> Result<RemoteEntry> {
        self.upload(path, local, progress).await
    }

    async fn remote_mkdir(&self, path: &RemotePath) -> Result<()> {
        self.mkdir(path).await
    }

    async fn remote_delete(&self, path: &RemotePath, permanent: bool) -> Result<()> {
        self.delete(path, permanent).await
    }

    async fn remote_rename(&self, from: &RemotePath, to: &RemotePath) -> Result<()> {
        self.rename(from, to).await
    }
}

/// The durable write-back spool: one file per path, named by the path hash.
#[derive(Debug, Clone)]
pub struct Spool {
    dir: PathBuf,
}

impl Spool {
    /// Open (creating) the spool directory.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir).map_err(|e| NidvueError::io(&dir, e))?;
        Ok(Self { dir })
    }

    fn file_for(&self, path: &RemotePath) -> PathBuf {
        self.dir
            .join(format!("{}.data", Hash::of(path.as_str().as_bytes())))
    }

    /// The spool file for a path, if one exists.
    pub fn path_of(&self, path: &RemotePath) -> Option<PathBuf> {
        let p = self.file_for(path);
        p.is_file().then_some(p)
    }

    /// Write bytes at an offset, creating the file if needed.
    pub async fn write_at(&self, path: &RemotePath, offset: u64, data: &[u8]) -> Result<u64> {
        let file_path = self.file_for(path);
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&file_path)
            .await
            .map_err(|e| NidvueError::io(&file_path, e))?;
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .map_err(|e| NidvueError::io(&file_path, e))?;
        file.write_all(data)
            .await
            .map_err(|e| NidvueError::io(&file_path, e))?;
        file.sync_all()
            .await
            .map_err(|e| NidvueError::io(&file_path, e))?;
        let len = file
            .metadata()
            .await
            .map_err(|e| NidvueError::io(&file_path, e))?
            .len();
        Ok(len)
    }

    /// Read a range from a spooled file.
    pub async fn read_at(&self, path: &RemotePath, offset: u64, size: usize) -> Result<Vec<u8>> {
        let file_path = self.file_for(path);
        let mut file = tokio::fs::File::open(&file_path)
            .await
            .map_err(|e| NidvueError::io(&file_path, e))?;
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .map_err(|e| NidvueError::io(&file_path, e))?;
        let mut buf = vec![0u8; size];
        let mut filled = 0usize;
        while filled < size {
            let n = file
                .read(&mut buf[filled..])
                .await
                .map_err(|e| NidvueError::io(&file_path, e))?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        buf.truncate(filled);
        Ok(buf)
    }

    /// Size of a spooled file.
    pub fn size_of(&self, path: &RemotePath) -> Option<u64> {
        std::fs::metadata(self.file_for(path)).ok().map(|m| m.len())
    }

    /// Truncate a spooled file.
    pub async fn truncate(&self, path: &RemotePath, size: u64) -> Result<()> {
        let file_path = self.file_for(path);
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&file_path)
            .await
            .map_err(|e| NidvueError::io(&file_path, e))?;
        file.set_len(size)
            .await
            .map_err(|e| NidvueError::io(&file_path, e))
    }

    /// Remove a spooled file.
    pub fn remove(&self, path: &RemotePath) -> Result<()> {
        match std::fs::remove_file(self.file_for(path)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(NidvueError::io(self.file_for(path), e)),
        }
    }

    /// Every spooled file, newest first (used by the drainer at startup).
    pub fn entries(&self) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&self.dir).map_err(|e| NidvueError::io(&self.dir, e))? {
            let entry = entry.map_err(|e| NidvueError::io(&self.dir, e))?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("data") {
                out.push(entry.path());
            }
        }
        out.sort();
        Ok(out)
    }
}

/// The engine-backed filesystem.
pub struct EngineVfs {
    remote: Arc<dyn Remote>,
    cache: Arc<Cache>,
    spool: Spool,
    root: RemotePath,
    read_only: bool,
    locks: Mutex<BTreeMap<String, Arc<Mutex<()>>>>,
    next_handle: std::sync::atomic::AtomicU64,
}

impl EngineVfs {
    /// Build the filesystem over a remote, a cache and a spool directory.
    pub fn new(
        remote: Arc<dyn Remote>,
        cache: Arc<Cache>,
        spool_dir: impl Into<PathBuf>,
    ) -> Result<Self> {
        let spool = Spool::open(spool_dir)?;
        Ok(Self {
            remote,
            cache,
            spool,
            root: RemotePath::root(),
            read_only: false,
            locks: Mutex::new(BTreeMap::new()),
            next_handle: std::sync::atomic::AtomicU64::new(1),
        })
    }

    /// Restrict the filesystem to a subtree of the mountpoint.
    pub fn with_root(mut self, root: RemotePath) -> Self {
        self.root = root;
        self
    }

    /// Mount read-only.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// The cache this filesystem serves from.
    pub fn cache(&self) -> &Arc<Cache> {
        &self.cache
    }

    /// The spool.
    pub fn spool(&self) -> &Spool {
        &self.spool
    }

    fn index(&self) -> &Arc<Index> {
        self.cache.index()
    }

    fn absolute(&self, path: &RemotePath) -> RemotePath {
        if self.root.is_root() {
            return path.clone();
        }
        match path.as_str() {
            "/" => self.root.clone(),
            other => RemotePath::parse(&format!("{}{}", self.root.as_str(), other))
                .unwrap_or_else(|_| self.root.clone()),
        }
    }

    async fn lock_for(&self, path: &str) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().await;
        map.entry(path.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn entry_to_meta(&self, row: &EntryRow) -> EntryMeta {
        let path = RemotePath::parse(&row.path).unwrap_or_else(|_| RemotePath::root());
        EntryMeta {
            path,
            kind: row.kind,
            version: row.version(),
            remote_rev: row.remote_rev.clone(),
        }
    }

    /// Metadata, from the index when possible, from the server otherwise.
    async fn entry(&self, path: &RemotePath) -> Result<EntryRow> {
        let abs = self.absolute(path);
        if let Some(row) = self.index().entry(abs.as_str())? {
            return Ok(row);
        }
        if abs.is_root() {
            return Ok(EntryRow {
                path: abs.as_str().into(),
                kind: EntryKind::Dir,
                size: 0,
                mtime_ms: 0,
                hash: None,
                remote_rev: None,
                dirty: false,
            });
        }
        let remote = self.remote.remote_stat(&abs).await?;
        if remote.is_incomplete() {
            return Err(NidvueError::NotFound(format!("{abs} (upload in progress)")));
        }
        let row = EntryRow {
            path: abs.as_str().into(),
            kind: remote.kind,
            size: remote.size,
            mtime_ms: remote.modified_ms,
            hash: remote.md5.clone().map(|m| format!("md5:{m}")),
            remote_rev: remote.state.clone(),
            dirty: false,
        };
        self.index().upsert_entry(&row)?;
        Ok(row)
    }

    /// Fetch a blob for a path into the cache and return its hash.
    async fn fetch(&self, path: &RemotePath, row: &EntryRow) -> Result<Hash> {
        let lock = self.lock_for(row.path.as_str()).await;
        let _guard = lock.lock().await;
        // Another task may have fetched it while we waited.
        if let Some(hash) = self.index().entry(&row.path)?.and_then(|r| r.hash) {
            if let Ok(parsed) = Hash::from_hex(&hash) {
                if self.cache.blobs().contains(&parsed) {
                    return Ok(parsed);
                }
            }
        }
        let tmp = self
            .spool
            .dir
            .join(format!(".fetch-{}", CorrelationId::new("dl")));
        let result = async {
            let mut file = tokio::fs::File::create(&tmp)
                .await
                .map_err(|e| NidvueError::io(&tmp, e))?;
            let size = self
                .remote
                .remote_download(&self.absolute(path), &mut file, Some(row.size), None)
                .await?;
            file.sync_all()
                .await
                .map_err(|e| NidvueError::io(&tmp, e))?;
            drop(file);
            if size != row.size {
                return Err(NidvueError::Verification(format!(
                    "{path}: downloaded {size} bytes, expected {}",
                    row.size
                )));
            }
            let (hash, stored) = self.cache.put_file(&tmp)?;
            if stored != row.size {
                return Err(NidvueError::Verification(format!(
                    "{path}: cache stored {stored} bytes"
                )));
            }
            Ok(hash)
        }
        .await;
        let _ = std::fs::remove_file(&tmp);
        let hash = result?;
        self.index().upsert_entry(&EntryRow {
            hash: Some(hash.as_str().to_string()),
            dirty: false,
            ..row.clone()
        })?;
        Ok(hash)
    }
}

#[async_trait]
impl Vfs for EngineVfs {
    async fn getattr(&self, path: &RemotePath) -> Result<EntryMeta> {
        let row = self.entry(path).await?;
        Ok(self.entry_to_meta(&row))
    }

    async fn readdir(
        &self,
        path: &RemotePath,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<EntryMeta>> {
        let abs = self.absolute(path);
        let mut children = self.index().list_dir(abs.as_str())?;
        let stale = self
            .index()
            .scan_cursor(abs.as_str())?
            .map(|at| now_ms() - at > 30_000)
            .unwrap_or(true);
        if stale {
            match self.remote.remote_list(&abs).await {
                Ok(listing) => {
                    for folder in &listing.folders {
                        let full = join_remote(&abs, &folder.path);
                        self.index().upsert_entry(&EntryRow {
                            path: full.as_str().into(),
                            kind: EntryKind::Dir,
                            size: 0,
                            mtime_ms: 0,
                            hash: None,
                            remote_rev: None,
                            dirty: false,
                        })?;
                    }
                    for file in &listing.files {
                        let full = join_remote(&abs, &file.name);
                        let existing = self.index().entry(full.as_str())?;
                        let dirty = existing.as_ref().map(|e| e.dirty).unwrap_or(false);
                        if dirty {
                            continue;
                        }
                        self.index().upsert_entry(&EntryRow {
                            path: full.as_str().into(),
                            kind: EntryKind::File,
                            size: file.size,
                            mtime_ms: file.modified_ms,
                            hash: file.md5.clone().map(|m| format!("md5:{m}")),
                            remote_rev: None,
                            dirty: false,
                        })?;
                    }
                    self.index().set_scan_cursor(abs.as_str(), now_ms(), None)?;
                    children = self.index().list_dir(abs.as_str())?;
                }
                Err(NidvueError::NotFound(_)) => {
                    return Err(NidvueError::NotFound(abs.to_string()))
                }
                Err(e) => {
                    warn!(path = %abs, error = %e, "listing failed; serving cached directory");
                }
            }
        }
        Ok(children
            .iter()
            .skip(offset)
            .take(limit)
            .map(|row| self.entry_to_meta(row))
            .collect())
    }

    async fn read(&self, path: &RemotePath, offset: u64, size: usize) -> Result<Vec<u8>> {
        let row = self.entry(path).await?;
        if row.kind == EntryKind::Dir {
            return Err(NidvueError::Unsupported("read on a directory".into()));
        }
        if row.dirty || self.spool.path_of(&self.absolute(path)).is_some() {
            return self.spool.read_at(&self.absolute(path), offset, size).await;
        }
        let hash = match row.hash.as_deref().and_then(|h| Hash::from_hex(h).ok()) {
            Some(h) if self.cache.blobs().contains(&h) => h,
            _ => self.fetch(path, &row).await?,
        };
        #[allow(clippy::type_complexity)]
        let _ = &hash;
        // Pin while reading so eviction cannot remove the blob underneath us.
        self.cache.pin(&hash)?;
        let path_display = path.to_string();
        let result = async {
            let mut file = self.cache.open_read(&hash)?;
            tokio::task::spawn_blocking(move || {
                use std::io::{Read, Seek, SeekFrom};
                file.seek(SeekFrom::Start(offset))
                    .map_err(|e| NidvueError::Io {
                        path: path_display.clone(),
                        source: e,
                    })?;
                let mut buf = vec![0u8; size];
                let mut filled = 0usize;
                while filled < size {
                    let n = file.read(&mut buf[filled..]).map_err(|e| NidvueError::Io {
                        path: path_display.clone(),
                        source: e,
                    })?;
                    if n == 0 {
                        break;
                    }
                    filled += n;
                }
                buf.truncate(filled);
                Ok::<_, NidvueError>(buf)
            })
            .await
            .map_err(|e| NidvueError::Internal(format!("read task: {e}")))?
        }
        .await;
        let _ = self.cache.unpin(&hash);
        result
    }

    async fn write(&self, path: &RemotePath, offset: u64, data: &[u8]) -> Result<u64> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let abs = self.absolute(path);
        let lock = self.lock_for(abs.as_str()).await;
        let _guard = lock.lock().await;
        let len = self.spool.write_at(&abs, offset, data).await?;
        let corr = CorrelationId::new("wr");
        self.index().journal_append(
            corr.as_str(),
            abs.as_str(),
            "upload",
            None,
            Some(len),
            None,
        )?;
        self.index().upsert_entry(&EntryRow {
            path: abs.as_str().into(),
            kind: EntryKind::File,
            size: len,
            mtime_ms: now_ms(),
            hash: None,
            remote_rev: None,
            dirty: true,
        })?;
        debug!(path = %abs, len, "buffered write to the spool");
        Ok(data.len() as u64)
    }

    async fn create(&self, path: &RemotePath, kind: EntryKind) -> Result<Handle> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let abs = self.absolute(path);
        let id = self
            .next_handle
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        match kind {
            EntryKind::Dir => {
                self.index().journal_append(
                    CorrelationId::new("mk").as_str(),
                    abs.as_str(),
                    "mkdir",
                    None,
                    None,
                    None,
                )?;
                self.index().upsert_entry(&EntryRow {
                    path: abs.as_str().into(),
                    kind: EntryKind::Dir,
                    size: 0,
                    mtime_ms: now_ms(),
                    hash: None,
                    remote_rev: None,
                    dirty: true,
                })?;
            }
            _ => {
                self.spool.write_at(&abs, 0, &[]).await?;
                self.index().journal_append(
                    CorrelationId::new("cr").as_str(),
                    abs.as_str(),
                    "upload",
                    None,
                    Some(0),
                    None,
                )?;
                self.index().upsert_entry(&EntryRow {
                    path: abs.as_str().into(),
                    kind: EntryKind::File,
                    size: 0,
                    mtime_ms: now_ms(),
                    hash: None,
                    remote_rev: None,
                    dirty: true,
                })?;
            }
        }
        Ok(Handle {
            id,
            path: abs,
            writable: true,
            hash: None,
        })
    }

    async fn unlink(&self, path: &RemotePath) -> Result<()> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let abs = self.absolute(path);
        let row = self.entry(&abs).await?;
        if row.kind == EntryKind::Dir && !self.index().list_dir(abs.as_str())?.is_empty() {
            return Err(NidvueError::Conflict {
                path: abs.to_string(),
                detail: "directory not empty".into(),
            });
        }
        self.index().journal_append(
            CorrelationId::new("rm").as_str(),
            abs.as_str(),
            "delete_remote",
            None,
            None,
            None,
        )?;
        self.spool.remove(&abs)?;
        self.index().delete_entry(abs.as_str())?;
        self.index().clear_baseline(abs.as_str())?;
        Ok(())
    }

    async fn rename(&self, from: &RemotePath, to: &RemotePath) -> Result<()> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let a = self.absolute(from);
        let b = self.absolute(to);
        // Deterministic lock order prevents two renames deadlocking.
        let (first, second) = if a.as_str() <= b.as_str() {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        };
        let l1 = self.lock_for(first.as_str()).await;
        let g1 = l1.lock().await;
        let l2 = self.lock_for(second.as_str()).await;
        let g2 = l2.lock().await;
        let row = self.entry(&a).await?;
        self.index().journal_append(
            CorrelationId::new("mv").as_str(),
            a.as_str(),
            "rename",
            None,
            None,
            Some(&serde_json::json!({ "to": b.as_str() }).to_string()),
        )?;
        // Move the spooled bytes if there are any, then the index rows.
        if let Some(spool_path) = self.spool.path_of(&a) {
            self.spool.write_at(&b, 0, &[]).await?;
            if let Some(target) = self.spool.path_of(&b) {
                std::fs::copy(&spool_path, &target).map_err(|e| NidvueError::io(&target, e))?;
            }
            self.spool.remove(&a)?;
        }
        self.index().delete_entry(a.as_str())?;
        self.index().upsert_entry(&EntryRow {
            path: b.as_str().into(),
            ..row
        })?;
        // Re-point children for a directory rename.
        if row.kind == EntryKind::Dir {
            for child in self.index().all_entries()? {
                if let Some(rest) = child.path.strip_prefix(&format!("{}/", a.as_str())) {
                    let new_path = format!("{}/{rest}", b.as_str());
                    self.index().delete_entry(&child.path)?;
                    self.index().upsert_entry(&EntryRow {
                        path: new_path,
                        ..child
                    })?;
                }
            }
        }
        drop(g2);
        drop(g1);
        Ok(())
    }

    async fn truncate(&self, path: &RemotePath, size: u64) -> Result<()> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let abs = self.absolute(path);
        if self.spool.path_of(&abs).is_none() {
            // Materialise the current content, then cut it.
            let row = self.entry(&abs).await?;
            if row.size > 0 {
                let bytes = self.read(path, 0, row.size as usize).await?;
                self.spool.write_at(&abs, 0, &bytes).await?;
            }
        }
        self.spool.truncate(&abs, size).await?;
        self.index().journal_append(
            CorrelationId::new("tr").as_str(),
            abs.as_str(),
            "upload",
            None,
            Some(size),
            None,
        )?;
        self.index().upsert_entry(&EntryRow {
            path: abs.as_str().into(),
            kind: EntryKind::File,
            size,
            mtime_ms: now_ms(),
            hash: None,
            remote_rev: None,
            dirty: true,
        })?;
        Ok(())
    }

    async fn statfs(&self) -> Result<FsStats> {
        let usage = self.cache.usage()?;
        Ok(FsStats {
            total_bytes: self.cache.capacity(),
            used_bytes: usage.bytes,
            free_bytes: self
                .cache
                .config()
                .max_size_bytes
                .saturating_sub(usage.bytes),
        })
    }

    async fn flush(&self, path: &RemotePath) -> Result<()> {
        let abs = self.absolute(path);
        if let Some(spool_path) = self.spool.path_of(&abs) {
            // fsync the spool file so a crash after this point cannot lose it.
            let file = tokio::fs::File::open(&spool_path)
                .await
                .map_err(|e| NidvueError::io(&spool_path, e))?;
            file.sync_all()
                .await
                .map_err(|e| NidvueError::io(&spool_path, e))?;
        }
        Ok(())
    }
}

fn join_remote(base: &RemotePath, relative: &str) -> RemotePath {
    let trimmed = relative.trim_matches('/');
    if trimmed.is_empty() {
        return base.clone();
    }
    RemotePath::parse(&format!(
        "{}/{}",
        base.as_str().trim_end_matches('/'),
        trimmed
    ))
    .unwrap_or_else(|_| base.clone())
}

/// Upload every spooled file through the remote, clearing the dirty flag on
/// success. This is the drain step the scheduler calls.
pub async fn drain_spool(vfs: &EngineVfs, limit: usize) -> Result<Vec<RemotePath>> {
    let mut drained = Vec::new();
    for row in vfs.index().all_entries()? {
        if !row.dirty || row.kind == EntryKind::Dir {
            continue;
        }
        let path = RemotePath::parse(&row.path)?;
        let Some(spool_path) = vfs.spool.path_of(&path) else {
            continue;
        };
        if let Some(parent) = path.parent() {
            vfs.remote.remote_mkdir(&parent).await?;
        }
        let remote = vfs.remote.remote_upload(&path, &spool_path, None).await?;
        let local = hash_file(&spool_path)?;
        if remote.size != local.size {
            warn!(path = %path, "remote size differs after upload");
            continue;
        }
        vfs.index().upsert_entry(&EntryRow {
            path: row.path.clone(),
            kind: EntryKind::File,
            size: local.size,
            mtime_ms: remote.modified_ms,
            hash: Some(local.hash.as_str().to_string()),
            remote_rev: remote.state.clone(),
            dirty: false,
        })?;
        vfs.index().set_baseline(
            &row.path,
            EntryKind::File,
            &Version::new(local.size, remote.modified_ms, Some(local.hash.clone())),
            &Version::new(remote.size, remote.modified_ms, Some(local.hash.clone())),
            now_ms(),
        )?;
        vfs.spool.remove(&path)?;
        drained.push(path);
        if drained.len() >= limit {
            break;
        }
    }
    Ok(drained)
}

/// One in-memory file: kind, bytes, mtime.
type MemEntry = (EntryKind, Vec<u8>, i64);

/// An in-memory filesystem for contract tests.
#[derive(Debug, Default)]
pub struct MemoryVfs {
    files: Mutex<BTreeMap<String, MemEntry>>,
    read_only: bool,
}

impl MemoryVfs {
    /// An empty in-memory filesystem.
    pub fn new() -> Self {
        Self::default()
    }

    /// A read-only in-memory filesystem.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

#[async_trait]
impl Vfs for MemoryVfs {
    async fn getattr(&self, path: &RemotePath) -> Result<EntryMeta> {
        let files = self.files.lock().await;
        match files.get(path.as_str()) {
            Some((kind, data, mtime)) => Ok(EntryMeta {
                path: path.clone(),
                kind: *kind,
                version: Version::new(data.len() as u64, *mtime, Some(Hash::of(data))),
                remote_rev: None,
            }),
            None => Err(NidvueError::NotFound(path.to_string())),
        }
    }

    async fn readdir(
        &self,
        path: &RemotePath,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<EntryMeta>> {
        let files = self.files.lock().await;
        let prefix = if path.is_root() {
            "/".to_string()
        } else {
            format!("{path}/")
        };
        let mut out: Vec<EntryMeta> = Vec::new();
        for (p, (kind, data, mtime)) in files.iter() {
            let Some(rest) = p.strip_prefix(&prefix) else {
                continue;
            };
            if rest.is_empty() || rest.contains('/') {
                continue;
            }
            out.push(EntryMeta {
                path: RemotePath::parse(p)?,
                kind: *kind,
                version: Version::new(data.len() as u64, *mtime, Some(Hash::of(data))),
                remote_rev: None,
            });
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out.into_iter().skip(offset).take(limit).collect())
    }

    async fn read(&self, path: &RemotePath, offset: u64, size: usize) -> Result<Vec<u8>> {
        let files = self.files.lock().await;
        let (_, data, _) = files
            .get(path.as_str())
            .ok_or_else(|| NidvueError::NotFound(path.to_string()))?;
        let start = (offset as usize).min(data.len());
        let end = (start + size).min(data.len());
        Ok(data[start..end].to_vec())
    }

    async fn write(&self, path: &RemotePath, offset: u64, data: &[u8]) -> Result<u64> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let mut files = self.files.lock().await;
        let entry = files
            .entry(path.as_str().to_string())
            .or_insert_with(|| (EntryKind::File, Vec::new(), now_ms()));
        let offset = offset as usize;
        if entry.1.len() < offset + data.len() {
            entry.1.resize(offset + data.len(), 0);
        }
        entry.1[offset..offset + data.len()].copy_from_slice(data);
        entry.2 = now_ms();
        Ok(data.len() as u64)
    }

    async fn create(&self, path: &RemotePath, kind: EntryKind) -> Result<Handle> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let mut files = self.files.lock().await;
        if files.contains_key(path.as_str()) {
            return Err(NidvueError::AlreadyExists(path.to_string()));
        }
        files.insert(path.as_str().to_string(), (kind, Vec::new(), now_ms()));
        Ok(Handle {
            id: 1,
            path: path.clone(),
            writable: true,
            hash: None,
        })
    }

    async fn unlink(&self, path: &RemotePath) -> Result<()> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let mut files = self.files.lock().await;
        if files.remove(path.as_str()).is_none() {
            return Err(NidvueError::NotFound(path.to_string()));
        }
        Ok(())
    }

    async fn rename(&self, from: &RemotePath, to: &RemotePath) -> Result<()> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let mut files = self.files.lock().await;
        let entry = files
            .remove(from.as_str())
            .ok_or_else(|| NidvueError::NotFound(from.to_string()))?;
        files.insert(to.as_str().to_string(), entry);
        Ok(())
    }

    async fn truncate(&self, path: &RemotePath, size: u64) -> Result<()> {
        if self.read_only {
            return Err(NidvueError::Mount("read-only filesystem".into()));
        }
        let mut files = self.files.lock().await;
        let entry = files
            .get_mut(path.as_str())
            .ok_or_else(|| NidvueError::NotFound(path.to_string()))?;
        entry.1.resize(size as usize, 0);
        Ok(())
    }

    async fn statfs(&self) -> Result<FsStats> {
        Ok(FsStats {
            total_bytes: 1 << 30,
            used_bytes: 0,
            free_bytes: 1 << 30,
        })
    }

    async fn flush(&self, _path: &RemotePath) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn vfs() -> MemoryVfs {
        let vfs = MemoryVfs::new();
        vfs.create(&RemotePath::parse("/a").unwrap(), EntryKind::Dir)
            .await
            .unwrap();
        vfs
    }

    #[tokio::test]
    async fn create_write_read_round_trip() {
        let vfs = vfs().await;
        let p = RemotePath::parse("/a/f.txt").unwrap();
        vfs.create(&p, EntryKind::File).await.unwrap();
        vfs.write(&p, 0, b"hello ").await.unwrap();
        vfs.write(&p, 6, b"world").await.unwrap();
        assert_eq!(vfs.read(&p, 0, 100).await.unwrap(), b"hello world");
        assert_eq!(vfs.read(&p, 6, 2).await.unwrap(), b"wo");
        let meta = vfs.getattr(&p).await.unwrap();
        assert_eq!(meta.version.size, 11);
    }

    #[tokio::test]
    async fn readdir_pages_and_sorts() {
        let vfs = vfs().await;
        for name in ["c", "a", "b"] {
            let p = RemotePath::parse(&format!("/a/{name}")).unwrap();
            vfs.create(&p, EntryKind::File).await.unwrap();
        }
        let page = vfs
            .readdir(&RemotePath::parse("/a").unwrap(), 0, 2)
            .await
            .unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].path.as_str(), "/a/a");
        let page2 = vfs
            .readdir(&RemotePath::parse("/a").unwrap(), 2, 2)
            .await
            .unwrap();
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].path.as_str(), "/a/c");
    }

    #[tokio::test]
    async fn rename_and_unlink() {
        let vfs = vfs().await;
        let p = RemotePath::parse("/a/f").unwrap();
        let q = RemotePath::parse("/a/g").unwrap();
        vfs.create(&p, EntryKind::File).await.unwrap();
        vfs.write(&p, 0, b"x").await.unwrap();
        vfs.rename(&p, &q).await.unwrap();
        assert!(vfs.getattr(&p).await.is_err());
        assert_eq!(vfs.read(&q, 0, 1).await.unwrap(), b"x");
        vfs.unlink(&q).await.unwrap();
        assert!(vfs.read(&q, 0, 1).await.is_err());
    }

    #[tokio::test]
    async fn read_only_refuses_mutation() {
        let vfs = MemoryVfs::new().read_only();
        let p = RemotePath::parse("/f").unwrap();
        assert!(matches!(
            vfs.write(&p, 0, b"x").await,
            Err(NidvueError::Mount(_))
        ));
        assert!(matches!(
            vfs.create(&p, EntryKind::File).await,
            Err(NidvueError::Mount(_))
        ));
    }

    #[tokio::test]
    async fn truncate_zeroes_and_extends() {
        let vfs = vfs().await;
        let p = RemotePath::parse("/a/f").unwrap();
        vfs.create(&p, EntryKind::File).await.unwrap();
        vfs.write(&p, 0, b"abcdef").await.unwrap();
        vfs.truncate(&p, 3).await.unwrap();
        assert_eq!(vfs.read(&p, 0, 10).await.unwrap(), b"abc");
        vfs.truncate(&p, 5).await.unwrap();
        assert_eq!(vfs.read(&p, 0, 10).await.unwrap(), b"abc\0\0");
    }

    #[tokio::test]
    async fn spool_is_durable_and_named_by_path_hash() {
        let dir = tempfile::tempdir().unwrap();
        let spool = Spool::open(dir.path()).unwrap();
        let p = RemotePath::parse("/x/y").unwrap();
        assert!(spool.path_of(&p).is_none());
        spool.write_at(&p, 0, b"abc").await.unwrap();
        assert_eq!(spool.size_of(&p), Some(3));
        assert_eq!(spool.read_at(&p, 1, 2).await.unwrap(), b"bc");
        assert!(spool.path_of(&p).is_some());
        spool.truncate(&p, 1).await.unwrap();
        assert_eq!(spool.size_of(&p), Some(1));
        spool.remove(&p).unwrap();
        assert!(spool.path_of(&p).is_none());
        assert_eq!(spool.entries().unwrap().len(), 0);
    }

    #[test]
    fn joins_are_segment_safe() {
        let base = RemotePath::parse("/Archive").unwrap();
        assert_eq!(join_remote(&base, "a/b").as_str(), "/Archive/a/b");
        assert_eq!(join_remote(&base, "/a").as_str(), "/Archive/a");
        assert_eq!(join_remote(&base, "").as_str(), "/Archive");
    }
}
