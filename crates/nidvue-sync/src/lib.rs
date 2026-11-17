// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The sync engine: journal-first execution, three-way reconciliation, conflict
//! copies, crash recovery and a debounced local watcher.
//!
//! ```no_run
//! # async fn run() -> nidvue_core::Result<()> {
//! use std::sync::Arc;
//! use nidvue_core::config::SyncConfig;
//! use nidvue_cache::Index;
//! use nidvue_sync::SyncEngine;
//! # let remote: Arc<dyn nidvue_vfs::Remote> = unimplemented!();
//! let index = Arc::new(Index::in_memory()?);
//! let (tx, _rx) = tokio::sync::broadcast::channel(64);
//! let engine = SyncEngine::new(remote, index, SyncConfig::default(), "/tmp/trash".into(), tx);
//! let report = engine.recover().await?;
//! println!("{} journal rows examined", report.examined);
//! # Ok(())
//! # }
//! ```

pub mod engine;
pub mod reconciler;
pub mod scanner;
pub mod scheduler;

pub use engine::{scan_remote, spawn_watcher, RecoveryReport, SyncEngine, SyncReport, Watcher};
pub use reconciler::{
    conflict_copy_path, decide_conflict, plan, plan_path, Action, ConflictDecision,
};
pub use scanner::{scan_local, PathFilter};
pub use scheduler::{Bandwidth, PathLocks, Scheduler};
