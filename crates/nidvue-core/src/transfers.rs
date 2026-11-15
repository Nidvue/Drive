// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The in-flight transfer list.
//!
//! The engine already announces every transfer on its event bus; this module
//! folds those events into a bounded list so `status`, the C ABI and the UI can
//! render one without keeping their own bookkeeping.
//!
//! The bound matters: a long sync can touch a million paths and a UI list that
//! grows with it is a memory leak with a scrollbar. Finished entries are evicted
//! oldest-first and an entry that is still running is never evicted.

use std::collections::VecDeque;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::event::{Direction, Event};
use crate::time::now_ms;

/// How many transfers the list keeps.
pub const MAX_TRANSFERS: usize = 512;

/// Where a transfer is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransferState {
    /// Waiting for a worker.
    Queued,
    /// Bytes are moving.
    Running,
    /// Finished and verified.
    Done,
    /// Stopped with an error.
    Failed,
}

impl TransferState {
    /// A stable string for the UI and for JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    /// True while the transfer still occupies a worker.
    pub fn is_live(&self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }
}

/// One row of the transfer list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Transfer {
    /// Stable key: direction and path, because one worker owns a path.
    pub id: String,
    /// Remote path being transferred.
    pub path: String,
    /// Direction of the transfer.
    pub direction: Direction,
    /// Current state.
    pub state: TransferState,
    /// Bytes completed.
    pub done: u64,
    /// Total bytes; 0 until the size is known.
    pub total: u64,
    /// Error text, already redacted.
    pub error: Option<String>,
    /// Whether the scheduler will retry.
    pub retryable: bool,
    /// When the row last changed, in Unix milliseconds.
    pub updated_ms: i64,
}

impl Transfer {
    /// Completion in `0..=1`; 0 when the total is unknown.
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.done as f64 / self.total as f64).clamp(0.0, 1.0)
        }
    }

    fn key(direction: Direction, path: &str) -> String {
        format!("{}:{path}", direction.as_str())
    }
}

/// The bounded list itself. Cheap to clone; every clone sees the same rows.
#[derive(Clone, Default)]
pub struct TransferRegistry {
    rows: std::sync::Arc<Mutex<VecDeque<Transfer>>>,
}

impl TransferRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one event into the list. Anything that is not a transfer event is
    /// ignored, so a caller can feed the whole bus through it.
    pub fn record(&self, event: &Event) {
        let mut rows = match self.rows.lock() {
            Ok(rows) => rows,
            Err(poisoned) => poisoned.into_inner(),
        };
        match event {
            Event::TransferQueued {
                path,
                direction,
                size,
            } => {
                upsert(&mut rows, *direction, path, |row| {
                    row.state = TransferState::Queued;
                    row.total = *size;
                    row.error = None;
                    row.retryable = false;
                });
            }
            Event::TransferProgress {
                path,
                direction,
                done,
                total,
            } => {
                upsert(&mut rows, *direction, path, |row| {
                    row.state = TransferState::Running;
                    row.done = *done;
                    row.total = *total;
                });
            }
            Event::TransferDone {
                path,
                direction,
                hash: _,
            } => {
                upsert(&mut rows, *direction, path, |row| {
                    row.state = TransferState::Done;
                    row.done = row.total.max(row.done);
                    row.error = None;
                });
            }
            Event::TransferFailed {
                path,
                direction,
                error,
                retryable,
            } => {
                upsert(&mut rows, *direction, path, |row| {
                    row.state = TransferState::Failed;
                    row.error = Some(error.clone());
                    row.retryable = *retryable;
                });
            }
            _ => {}
        }
        trim(&mut rows);
    }

    /// Every row, oldest first.
    pub fn list(&self) -> Vec<Transfer> {
        match self.rows.lock() {
            Ok(rows) => rows.iter().cloned().collect(),
            Err(poisoned) => poisoned.into_inner().iter().cloned().collect(),
        }
    }

    /// Rows that still occupy a worker.
    pub fn live(&self) -> Vec<Transfer> {
        self.list()
            .into_iter()
            .filter(|t| t.state.is_live())
            .collect()
    }

    /// How many rows are queued.
    pub fn queued(&self) -> u64 {
        self.count(TransferState::Queued)
    }

    /// How many rows are running.
    pub fn running(&self) -> u64 {
        self.count(TransferState::Running)
    }

    /// How many rows are recorded.
    pub fn len(&self) -> usize {
        match self.rows.lock() {
            Ok(rows) => rows.len(),
            Err(poisoned) => poisoned.into_inner().len(),
        }
    }

    /// True when no transfer has been seen.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Forget everything; the UI calls this after the user dismisses the list.
    pub fn clear(&self) {
        match self.rows.lock() {
            Ok(mut rows) => rows.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }

    fn count(&self, state: TransferState) -> u64 {
        self.list().iter().filter(|t| t.state == state).count() as u64
    }
}

fn upsert(
    rows: &mut VecDeque<Transfer>,
    direction: Direction,
    path: &str,
    update: impl FnOnce(&mut Transfer),
) {
    let id = Transfer::key(direction, path);
    if let Some(row) = rows.iter_mut().find(|r| r.id == id) {
        update(row);
        row.updated_ms = now_ms();
        return;
    }
    let mut row = Transfer {
        id,
        path: path.to_string(),
        direction,
        state: TransferState::Queued,
        done: 0,
        total: 0,
        error: None,
        retryable: false,
        updated_ms: now_ms(),
    };
    update(&mut row);
    rows.push_back(row);
}

fn trim(rows: &mut VecDeque<Transfer>) {
    while rows.len() > MAX_TRANSFERS {
        match rows.iter().position(|r| !r.state.is_live()) {
            Some(index) => {
                rows.remove(index);
            }
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued(path: &str) -> Event {
        Event::TransferQueued {
            path: path.into(),
            size: 1024,
            direction: Direction::Upload,
        }
    }

    #[test]
    fn a_transfer_walks_queued_running_done() {
        let reg = TransferRegistry::new();
        reg.record(&queued("/a"));
        let row = &reg.list()[0];
        assert_eq!(row.state, TransferState::Queued);
        assert_eq!(row.id, "upload:/a");

        reg.record(&Event::TransferProgress {
            path: "/a".into(),
            direction: Direction::Upload,
            done: 5,
            total: 10,
        });
        let row = &reg.list()[0];
        assert_eq!(row.state, TransferState::Running);
        assert_eq!(row.fraction(), 0.5);
        assert_eq!(reg.running(), 1);
        assert_eq!(reg.queued(), 0);

        reg.record(&Event::TransferDone {
            path: "/a".into(),
            direction: Direction::Upload,
            hash: "abc".into(),
        });
        let row = &reg.list()[0];
        assert_eq!(row.state, TransferState::Done);
        assert_eq!(row.done, 10);
        assert!(reg.live().is_empty());
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn a_failure_keeps_its_error_and_retry_flag() {
        let reg = TransferRegistry::new();
        reg.record(&Event::TransferFailed {
            path: "/b".into(),
            direction: Direction::Download,
            error: "network down".into(),
            retryable: true,
        });
        let row = &reg.list()[0];
        assert_eq!(row.state, TransferState::Failed);
        assert_eq!(row.error.as_deref(), Some("network down"));
        assert!(row.retryable);
        assert_eq!(row.id, "download:/b");
    }

    #[test]
    fn the_two_directions_are_separate_rows() {
        let reg = TransferRegistry::new();
        reg.record(&queued("/a"));
        reg.record(&Event::TransferQueued {
            path: "/a".into(),
            size: 1024,
            direction: Direction::Download,
        });
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn the_list_is_bounded_and_keeps_live_rows() {
        let reg = TransferRegistry::new();
        reg.record(&Event::TransferProgress {
            path: "/live".into(),
            direction: Direction::Upload,
            done: 1,
            total: 2,
        });
        for i in 0..MAX_TRANSFERS + 50 {
            reg.record(&Event::TransferDone {
                path: format!("/done/{i}"),
                direction: Direction::Upload,
                hash: "h".into(),
            });
        }
        assert_eq!(reg.len(), MAX_TRANSFERS);
        assert_eq!(reg.running(), 1);
        assert!(reg.list().iter().any(|t| t.path == "/live"));
        assert!(reg
            .list()
            .iter()
            .any(|t| t.path == format!("/done/{}", MAX_TRANSFERS + 49)));
        assert!(!reg.list().iter().any(|t| t.path == "/done/0"));
    }

    #[test]
    fn unrelated_events_are_ignored() {
        let reg = TransferRegistry::new();
        reg.record(&Event::Health {
            state: crate::event::HealthState::Ok,
            detail: None,
        });
        assert!(reg.is_empty());
    }

    #[test]
    fn clear_empties_the_list() {
        let reg = TransferRegistry::new();
        reg.record(&queued("/a"));
        assert_eq!(reg.len(), 1);
        reg.clear();
        assert!(reg.is_empty());
    }

    #[test]
    fn a_clone_shares_the_rows() {
        let reg = TransferRegistry::new();
        let other = reg.clone();
        other.record(&queued("/a"));
        assert_eq!(reg.len(), 1);
    }
}
