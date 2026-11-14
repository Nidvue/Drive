// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Scheduling: per-path serialisation and a shared bandwidth budget.
//!
//! Two workers never operate on one path. Locks are keyed by path and taken in
//! a deterministic order by callers that need two (rename), which is what keeps
//! the scheme deadlock-free. Bandwidth is a single token bucket shared by every
//! transfer, so a cap is a cap and not a suggestion.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nidvue_core::config::SyncConfig;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

/// Keyed async locks, one per path.
#[derive(Default)]
pub struct PathLocks {
    locks: Mutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

impl PathLocks {
    /// Lock one path. The guard releases the path when dropped.
    pub async fn lock(&self, path: &str) -> OwnedMutexGuard<()> {
        let lock = {
            let mut map = self.locks.lock().unwrap_or_else(|e| e.into_inner());
            map.entry(path.to_string())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };
        lock.lock_owned().await
    }

    /// Number of tracked paths (for diagnostics).
    pub fn len(&self) -> usize {
        self.locks.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// True when no path is tracked.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A shared bandwidth budget in bytes per second. `0` means unlimited.
pub struct Bandwidth {
    rate: f64,
    state: Mutex<(f64, Instant)>,
}

impl Bandwidth {
    /// Build a limiter.
    pub fn new(bytes_per_second: u64) -> Self {
        Self {
            rate: bytes_per_second as f64,
            state: Mutex::new((0.0, Instant::now())),
        }
    }

    /// True when a cap is configured.
    pub fn is_limited(&self) -> bool {
        self.rate > 0.0
    }

    /// Account for `bytes`, sleeping when the budget is exhausted.
    pub async fn account(&self, bytes: u64) {
        if self.rate <= 0.0 {
            return;
        }
        let wait = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let now = Instant::now();
            let elapsed = now.saturating_duration_since(state.1).as_secs_f64();
            state.1 = now;
            state.0 = (state.0 + elapsed * self.rate - bytes as f64).max(-self.rate);
            if state.0 < 0.0 {
                Duration::from_secs_f64(-state.0 / self.rate)
            } else {
                Duration::ZERO
            }
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

/// The scheduler shared by every action of an engine.
pub struct Scheduler {
    locks: PathLocks,
    bandwidth: Bandwidth,
    workers: usize,
}

impl Scheduler {
    /// Build a scheduler from the sync configuration.
    pub fn new(cfg: SyncConfig) -> Self {
        Self {
            locks: PathLocks::default(),
            bandwidth: Bandwidth::new(cfg.bandwidth_limit_bps),
            workers: cfg.workers,
        }
    }

    /// Take the lock for one path.
    pub async fn lock_path(&self, path: &str) -> OwnedMutexGuard<()> {
        self.locks.lock(path).await
    }

    /// Account for transferred bytes against the shared cap.
    pub async fn throttle(&self, bytes: u64) {
        self.bandwidth.account(bytes).await;
    }

    /// The configured worker count.
    pub fn workers(&self) -> usize {
        self.workers
    }

    /// True when a bandwidth cap is active.
    pub fn bandwidth_limited(&self) -> bool {
        self.bandwidth.is_limited()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn one_path_is_never_worked_on_twice_at_once() {
        let locks = Arc::new(PathLocks::default());
        let inside = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let locks = locks.clone();
            let inside = inside.clone();
            let max_seen = max_seen.clone();
            tasks.push(tokio::spawn(async move {
                let _guard = locks.lock("/same/path").await;
                let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
                max_seen.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(5)).await;
                inside.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for t in tasks {
            t.await.unwrap();
        }
        assert_eq!(max_seen.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn different_paths_run_concurrently() {
        let locks = Arc::new(PathLocks::default());
        let inside = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for i in 0..4 {
            let locks = locks.clone();
            let inside = inside.clone();
            let max_seen = max_seen.clone();
            tasks.push(tokio::spawn(async move {
                let _guard = locks.lock(&format!("/p/{i}")).await;
                let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
                max_seen.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                inside.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for t in tasks {
            t.await.unwrap();
        }
        assert!(max_seen.load(Ordering::SeqCst) > 1);
    }

    #[tokio::test]
    async fn bandwidth_cap_delays_transfers() {
        let bw = Bandwidth::new(1_000); // 1 KB/s
        bw.account(500).await; // within the first second
        let start = Instant::now();
        bw.account(1_000).await;
        assert!(
            start.elapsed() >= Duration::from_millis(400),
            "{:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn unlimited_bandwidth_never_waits() {
        let bw = Bandwidth::new(0);
        assert!(!bw.is_limited());
        let start = Instant::now();
        bw.account(1_000_000_000).await;
        assert!(start.elapsed() < Duration::from_millis(50));
    }
}
