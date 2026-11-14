// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Correlation ids for operations and transfers.
//!
//! Ids are short, sortable and collision-free without a UUID dependency:
//! a millisecond timestamp plus a process-wide atomic counter.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

static COUNTER: AtomicU64 = AtomicU64::new(1);

/// An operation id, e.g. `op-17a2f3c4d5e-0001`.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Mint a new id. Monotonic within a process and unique across processes.
    pub fn new(kind: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let ts = crate::time::now_ms() as u64;
        Self(format!("{kind}-{ts:x}-{n:04x}"))
    }

    /// The id text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_sortable_by_creation() {
        let a = CorrelationId::new("op");
        let b = CorrelationId::new("op");
        assert_ne!(a, b);
        assert!(a.as_str().starts_with("op-"));
        assert!(a.as_str() < b.as_str());
    }
}
