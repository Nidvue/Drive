// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Typed events.
//!
//! Every consumer - the Qt UI through the FFI bridge, the CLI, plugins, the
//! Prometheus exporter - reads the same enum. Events are serialised with a
//! `type` tag and a `CorrelationId`, so a transfer can be traced from the log
//! to the UI.

use serde::{Deserialize, Serialize};

use crate::conflict::Conflict;
use crate::id::CorrelationId;

/// Which way bytes are moving.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Local to Jottacloud.
    Upload,
    /// Jottacloud to local.
    Download,
}

/// Overall engine health, surfaced in the tray and the dashboard.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    /// Everything is working.
    #[default]
    Ok,
    /// Working, but degraded (offline queue, failed transfers, conflicts).
    Degraded,
    /// The network is unreachable; cached content is served.
    Offline,
    /// A fatal error needs attention.
    Error,
}

/// Something that happened in the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// A sync pass began.
    SyncStarted {
        /// Pair name.
        pair: String,
    },
    /// A sync pass finished.
    SyncFinished {
        /// Pair name.
        pair: String,
        /// Entries observed.
        scanned: u64,
        /// Actions planned.
        planned: u64,
        /// Actions applied.
        applied: u64,
        /// Conflicts detected in this pass.
        conflicts: u64,
    },
    /// A transfer was queued.
    TransferQueued {
        /// Path being transferred.
        path: String,
        /// Direction of the transfer.
        direction: Direction,
    },
    /// A transfer made progress.
    TransferProgress {
        /// Path being transferred.
        path: String,
        /// Direction of the transfer.
        direction: Direction,
        /// Bytes completed.
        done: u64,
        /// Total bytes.
        total: u64,
    },
    /// A transfer completed and was verified.
    TransferDone {
        /// Path transferred.
        path: String,
        /// Direction of the transfer.
        direction: Direction,
        /// Content hash of the transferred bytes.
        hash: String,
    },
    /// A transfer failed.
    TransferFailed {
        /// Path that failed.
        path: String,
        /// Direction of the transfer.
        direction: Direction,
        /// Error text, already redacted.
        error: String,
        /// Whether the scheduler will retry it.
        retryable: bool,
    },
    /// A conflict was detected.
    ConflictDetected {
        /// The conflict record.
        conflict: Conflict,
    },
    /// A conflict was resolved.
    ConflictResolved {
        /// Conflict id.
        id: String,
        /// Resolution chosen.
        resolution: String,
    },
    /// Cache accounting changed.
    CacheStats {
        /// Bytes stored.
        bytes: u64,
        /// Blobs stored.
        blobs: u64,
        /// Configured capacity.
        capacity: u64,
    },
    /// Health changed.
    Health {
        /// New health state.
        state: HealthState,
        /// Why, when not `Ok`.
        detail: Option<String>,
    },
    /// A plugin was loaded.
    PluginLoaded {
        /// Plugin id.
        id: String,
        /// Plugin version.
        version: String,
    },
    /// A plugin was rejected or trapped.
    PluginFailed {
        /// Plugin id.
        id: String,
        /// Reason.
        error: String,
    },
    /// A mount became available at a path.
    MountReady {
        /// Mountpoint path.
        path: String,
    },
    /// A mount was unmounted.
    MountStopped {
        /// Mountpoint path.
        path: String,
    },
    /// A periodic snapshot for the dashboard.
    Stats(StatsSnapshot),
    /// The engine is shutting down.
    Shutdown,
}

impl Event {
    /// A short type name, used for metrics labels and log fields.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SyncStarted { .. } => "sync_started",
            Self::SyncFinished { .. } => "sync_finished",
            Self::TransferQueued { .. } => "transfer_queued",
            Self::TransferProgress { .. } => "transfer_progress",
            Self::TransferDone { .. } => "transfer_done",
            Self::TransferFailed { .. } => "transfer_failed",
            Self::ConflictDetected { .. } => "conflict_detected",
            Self::ConflictResolved { .. } => "conflict_resolved",
            Self::CacheStats { .. } => "cache_stats",
            Self::Health { .. } => "health",
            Self::PluginLoaded { .. } => "plugin_loaded",
            Self::PluginFailed { .. } => "plugin_failed",
            Self::MountReady { .. } => "mount_ready",
            Self::MountStopped { .. } => "mount_stopped",
            Self::Stats(_) => "stats",
            Self::Shutdown => "shutdown",
        }
    }
}

/// An event with its correlation id and timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EventEnvelope {
    /// Correlates the event with the operation that produced it.
    pub id: CorrelationId,
    /// Milliseconds since the epoch.
    pub ts_ms: i64,
    /// The event itself.
    pub event: Event,
}

impl EventEnvelope {
    /// Wrap an event with a fresh correlation id.
    pub fn new(event: Event) -> Self {
        Self {
            id: CorrelationId::new("ev"),
            ts_ms: crate::time::now_ms(),
            event,
        }
    }
}

/// A point-in-time snapshot for the dashboard, the tray and `status --json`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StatsSnapshot {
    /// Upload throughput, bytes/second, exponentially smoothed.
    pub upload_bps: u64,
    /// Download throughput, bytes/second, exponentially smoothed.
    pub download_bps: u64,
    /// Transfers waiting to run.
    pub queue_depth: u64,
    /// Transfers running now.
    pub active_transfers: u64,
    /// Bytes in the blob cache.
    pub cache_bytes: u64,
    /// Configured cache capacity.
    pub cache_capacity: u64,
    /// Entries in the metadata index.
    pub index_entries: u64,
    /// Conflicts awaiting a user decision.
    pub conflicts_open: u64,
    /// Engine health.
    pub health: HealthState,
    /// Seconds since the engine started.
    pub uptime_s: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_round_trip_through_json() {
        let e = Event::TransferProgress {
            path: "/a/b".into(),
            direction: Direction::Upload,
            done: 10,
            total: 20,
        };
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("\"type\":\"transfer_progress\""));
        let back: Event = serde_json::from_str(&j).unwrap();
        assert_eq!(back, e);
        assert_eq!(back.kind(), "transfer_progress");
    }

    #[test]
    fn envelopes_carry_a_correlation_id() {
        let env = EventEnvelope::new(Event::Shutdown);
        assert!(env.id.as_str().starts_with("ev-"));
        assert!(env.ts_ms > 0);
    }
}
