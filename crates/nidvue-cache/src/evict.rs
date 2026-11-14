// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Eviction: segmented LRU-K, watermarks and an exact hard cap.
//!
//! Eviction never removes a pinned, dirty or open blob (open handles pin), and
//! it runs in bounded batches so it does not stop the world between I/O calls.

use nidvue_core::config::{CacheConfig, EvictionPolicy};
use nidvue_core::error::Result;

use crate::blob::BlobStore;
use crate::index::Index;

/// What one eviction pass did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvictionOutcome {
    /// Blobs removed.
    pub removed: u64,
    /// Bytes freed.
    pub freed_bytes: u64,
    /// True when the target could not be reached (everything left is pinned).
    pub still_over: bool,
}

/// The eviction planner, shared by the cache facade and the tests.
pub struct Evictor<'a> {
    index: &'a Index,
    blobs: &'a BlobStore,
    policy: EvictionPolicy,
}

impl<'a> Evictor<'a> {
    /// Build an evictor over an index and a blob store.
    pub fn new(index: &'a Index, blobs: &'a BlobStore, policy: EvictionPolicy) -> Self {
        Self {
            index,
            blobs,
            policy,
        }
    }

    /// Evict until the cache holds at most `target_bytes`, in batches of at
    /// most `batch` blobs.
    pub fn evict_to(&self, target_bytes: u64, batch: usize) -> Result<EvictionOutcome> {
        let mut outcome = EvictionOutcome::default();
        if self.policy == EvictionPolicy::None {
            let usage = self.index.cache_usage()?;
            outcome.still_over = usage.bytes > target_bytes;
            return Ok(outcome);
        }
        loop {
            let usage = self.index.cache_usage()?;
            if usage.bytes <= target_bytes {
                return Ok(outcome);
            }
            let candidates = self.index.eviction_candidates(batch.max(1))?;
            if candidates.is_empty() {
                // Everything left is pinned or dirty: honest failure, not a loop.
                outcome.still_over = true;
                return Ok(outcome);
            }
            for (hash, size) in candidates {
                if let Ok(parsed) = nidvue_core::hash::Hash::from_hex(hash.clone()) {
                    self.blobs.remove(&parsed)?;
                }
                self.index.remove_blob(&hash)?;
                outcome.removed += 1;
                outcome.freed_bytes += size;
            }
        }
    }

    /// Make room for `incoming` bytes: evict to the low watermark when the
    /// high watermark would be crossed. Returns an error when the hard cap
    /// cannot be honoured even after eviction.
    pub fn ensure_room(&self, incoming: u64, cfg: &CacheConfig) -> Result<EvictionOutcome> {
        let usage = self.index.cache_usage()?;
        let high = (cfg.max_size_bytes as f64 * cfg.high_watermark) as u64;
        if usage.bytes.saturating_add(incoming) <= high {
            return Ok(EvictionOutcome::default());
        }
        let low = (cfg.max_size_bytes as f64 * cfg.low_watermark) as u64;
        let target = low.saturating_sub(incoming);
        let outcome = self.evict_to(target, 32)?;
        let after = self.index.cache_usage()?;
        if after.bytes.saturating_add(incoming) > cfg.max_size_bytes {
            return Err(nidvue_core::NidvueError::Cache(format!(
                "cache is full: {} bytes used, {} requested, {} cap",
                after.bytes, incoming, cfg.max_size_bytes
            )));
        }
        Ok(outcome)
    }
}
