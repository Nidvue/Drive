// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The cache: a content-addressed blob store plus the SQLite metadata index,
//! with segmented LRU-K eviction and integrity scrubbing.
//!
//! Reads are read-through (a warm blob never touches the network), writes land
//! here and are drained by the sync engine. The index is the only authority on
//! what a path currently is.

pub mod blob;
pub mod evict;
pub mod index;

use std::path::Path;
use std::sync::Arc;

use nidvue_core::config::CacheConfig;
use nidvue_core::error::{NidvueError, Result};
use nidvue_core::hash::Hash;
use nidvue_core::time::now_ms;
use tracing::{debug, warn};

pub use blob::BlobStore;
pub use evict::{EvictionOutcome, Evictor};
pub use index::{CacheUsage, ConflictRow, EntryRow, Index, JournalRow};

/// The cache facade.
pub struct Cache {
    blobs: BlobStore,
    index: Arc<Index>,
    cfg: CacheConfig,
}

impl Cache {
    /// Open the cache under a state directory.
    pub fn open(cfg: &CacheConfig, blobs_dir: &Path, index_path: &Path) -> Result<Self> {
        let blobs = BlobStore::open(blobs_dir)?;
        let index = Arc::new(Index::open(index_path)?);
        Ok(Self { blobs, index, cfg: cfg.clone() })
    }

    /// An in-memory cache rooted in a temporary directory (tests, previews).
    pub fn temporary(cfg: &CacheConfig) -> Result<(Self, tempfile::TempDir)> {
        let dir = tempfile::tempdir().map_err(|e| NidvueError::Cache(e.to_string()))?;
        let cache = Self::open(cfg, &dir.path().join("blobs"), &dir.path().join("index.sqlite"))?;
        Ok((cache, dir))
    }

    /// The metadata index.
    pub fn index(&self) -> &Arc<Index> {
        &self.index
    }

    /// The blob store.
    pub fn blobs(&self) -> &BlobStore {
        &self.blobs
    }

    /// The active configuration.
    pub fn config(&self) -> &CacheConfig {
        &self.cfg
    }

    /// Replace the hot-reloadable configuration.
    pub fn reconfigure(&mut self, cfg: CacheConfig) {
        self.cfg = cfg;
    }

    /// Store bytes, evicting first if the hard cap would be exceeded.
    pub fn put(&self, bytes: &[u8]) -> Result<Hash> {
        Evictor::new(&self.index, &self.blobs, self.cfg.eviction_policy)
            .ensure_room(bytes.len() as u64, &self.cfg)?;
        let hash = self.blobs.put(bytes)?;
        self.index.record_blob(hash.as_str(), bytes.len() as u64, now_ms())?;
        Ok(hash)
    }

    /// Copy a local file into the cache, hashing while copying.
    pub fn put_file(&self, source: &Path) -> Result<(Hash, u64)> {
        let size = std::fs::metadata(source).map_err(|e| NidvueError::io(source, e))?.len();
        Evictor::new(&self.index, &self.blobs, self.cfg.eviction_policy)
            .ensure_room(size, &self.cfg)?;
        let got = self.blobs.put_file(source)?;
        self.index.record_blob(got.hash.as_str(), got.size, now_ms())?;
        Ok((got.hash, got.size))
    }

    /// Read a blob and record the access.
    ///
    /// The hash was verified when the blob was written and is verified again by
    /// the background scrubber; a read does not re-hash on every call, because
    /// that would put the cache at the speed of the CPU rather than the disk.
    pub fn read(&self, hash: &Hash) -> Result<Vec<u8>> {
        let bytes = self.blobs.read(hash)?;
        self.index.touch_blob(hash.as_str(), now_ms())?;
        Ok(bytes)
    }

    /// Open a blob for streaming and record the access.
    pub fn open_read(&self, hash: &Hash) -> Result<std::fs::File> {
        let file = self.blobs.open_read(hash)?;
        self.index.touch_blob(hash.as_str(), now_ms())?;
        Ok(file)
    }

    /// True when the blob is present and its bytes still hash to its name.
    pub fn is_intact(&self, hash: &Hash) -> Result<bool> {
        self.blobs.verify(hash)
    }

    /// Pin a blob (an open handle). Pinned blobs are never evicted.
    pub fn pin(&self, hash: &Hash) -> Result<()> {
        self.index.pin(hash.as_str())
    }

    /// Release one pin.
    pub fn unpin(&self, hash: &Hash) -> Result<()> {
        self.index.unpin(hash.as_str())
    }

    /// Mark a blob as not yet drained to the cloud.
    pub fn set_dirty(&self, hash: &Hash, dirty: bool) -> Result<()> {
        self.index.set_dirty(hash.as_str(), dirty)
    }

    /// Evict down to the low watermark.
    pub fn evict_to_capacity(&self) -> Result<EvictionOutcome> {
        let low = (self.cfg.max_size_bytes as f64 * self.cfg.low_watermark) as u64;
        Evictor::new(&self.index, &self.blobs, self.cfg.eviction_policy).evict_to(low, 32)
    }

    /// Aggregate counters for the UI and `status --json`.
    pub fn usage(&self) -> Result<CacheUsage> {
        self.index.cache_usage()
    }

    /// Verify every blob, delete the corrupt ones and return what was found.
    ///
    /// A corrupt blob is never served: the caller re-fetches it.
    pub fn scrub(&self) -> Result<ScrubReport> {
        let mut report = ScrubReport::default();
        for (hash_text, size) in self.index.eviction_candidates(i64::MAX as usize)? {
            report.checked += 1;
            let Ok(hash) = Hash::from_hex(hash_text.clone()) else {
                self.index.remove_blob(&hash_text)?;
                report.removed += 1;
                continue;
            };
            match self.blobs.verify(&hash) {
                Ok(true) => {}
                Ok(false) => {
                    warn!(%hash, "corrupt blob removed by scrub");
                    self.blobs.remove(&hash)?;
                    self.index.remove_blob(&hash_text)?;
                    report.removed += 1;
                    report.corrupt_bytes += size;
                }
                Err(e) => {
                    debug!(%hash, error = %e, "blob unreadable during scrub");
                    report.removed += 1;
                }
            }
        }
        Ok(report)
    }

    /// Free space on the filesystem holding the cache, when the platform can
    /// report it.
    pub fn free_disk_bytes(&self) -> u64 {
        free_disk_bytes(self.blobs.root())
    }

    /// The effective capacity: the configured cap, or less if the disk is
    /// running out of the configured reserve.
    pub fn effective_capacity(&self) -> u64 {
        let free = self.free_disk_bytes();
        let usable = free.saturating_sub(self.cfg.min_free_disk_bytes);
        self.cfg.max_size_bytes.min(usable.max(self.cfg.min_free_disk_bytes))
    }
}

/// What a scrub found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScrubReport {
    /// Blobs examined.
    pub checked: u64,
    /// Blobs removed (corrupt or unreadable).
    pub removed: u64,
    /// Bytes reclaimed.
    pub corrupt_bytes: u64,
}

/// Free bytes on the filesystem containing `path`.
#[cfg(unix)]
pub fn free_disk_bytes(path: &Path) -> u64 {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let Ok(cpath) = CString::new(path.as_os_str().as_bytes()) else {
        return u64::MAX;
    };
    // SAFETY: `statvfs` writes only into the struct we provide and the path is
    // a valid NUL-terminated string.
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(cpath.as_ptr(), &mut stat) == 0 {
            (stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64)
        } else {
            u64::MAX
        }
    }
}

/// Free bytes: not implemented on this platform, so the configured cap rules.
#[cfg(not(unix))]
pub fn free_disk_bytes(_path: &Path) -> u64 {
    u64::MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_cache(max: u64) -> (Cache, tempfile::TempDir) {
        let cfg = CacheConfig {
            max_size_bytes: max,
            min_free_disk_bytes: 0,
            ..Default::default()
        };
        Cache::temporary(&cfg).unwrap()
    }

    #[test]
    fn put_read_and_usage() {
        let (cache, _dir) = small_cache(1024);
        let hash = cache.put(b"hello").unwrap();
        assert_eq!(cache.read(&hash).unwrap(), b"hello");
        let usage = cache.usage().unwrap();
        assert_eq!(usage.bytes, 5);
        assert_eq!(usage.blobs, 1);
    }

    #[test]
    fn hard_cap_is_enforced() {
        let (cache, _dir) = small_cache(64);
        cache.put(&[1u8; 32]).unwrap();
        cache.put(&[2u8; 32]).unwrap();
        // The third write would exceed the cap; old blobs are evicted instead.
        let third = cache.put(&[3u8; 32]).unwrap();
        assert!(cache.usage().unwrap().bytes <= 64);
        assert!(cache.read(&third).is_ok());
    }

    #[test]
    fn a_pinned_blob_is_never_evicted_and_the_cap_is_honest() {
        let (cache, _dir) = small_cache(64);
        let pinned = cache.put(&[9u8; 32]).unwrap();
        cache.pin(&pinned).unwrap();
        let other = cache.put(&[8u8; 32]).unwrap();
        cache.pin(&other).unwrap();
        let err = cache.put(&[7u8; 32]).unwrap_err();
        assert!(err.to_string().contains("cache is full"), "{err}");
        // The pinned blobs are still readable.
        assert_eq!(cache.read(&pinned).unwrap().len(), 32);
    }

    #[test]
    fn dirty_blobs_are_not_evicted() {
        let (cache, _dir) = small_cache(64);
        let dirty = cache.put(&[5u8; 32]).unwrap();
        cache.set_dirty(&dirty, true).unwrap();
        let clean = cache.put(&[6u8; 32]).unwrap();
        assert_ne!(dirty, clean);
        // Only the clean blob is a candidate.
        let candidates = cache.index().eviction_candidates(10).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, clean.as_str());
    }

    #[test]
    fn scrub_removes_corrupt_blobs() {
        let (cache, _dir) = small_cache(1024);
        let good = cache.put(b"good").unwrap();
        let bad = cache.put(b"bad").unwrap();
        std::fs::write(cache.blobs().path_of(&bad), b"tampered").unwrap();
        let report = cache.scrub().unwrap();
        assert_eq!(report.checked, 2);
        assert_eq!(report.removed, 1);
        assert!(cache.read(&good).is_ok());
        assert!(cache.read(&bad).is_err());
        assert_eq!(cache.usage().unwrap().blobs, 1);
    }

    #[test]
    fn free_disk_is_reported() {
        let (cache, _dir) = small_cache(1024);
        assert!(cache.free_disk_bytes() > 0);
        assert!(cache.effective_capacity() > 0);
    }
}
