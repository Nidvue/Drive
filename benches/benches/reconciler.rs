// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Throughput of the three-way planner: the engine's hot loop.

use std::collections::BTreeMap;
use std::time::Instant;

use nidvue_core::hash::Hash;
use nidvue_core::path::RemotePath;
use nidvue_core::types::{BaselineEntry, EntryKind, Version};
use nidvue_sync::plan;

fn main() {
    let n = 100_000usize;
    let mut local = BTreeMap::new();
    let mut remote = BTreeMap::new();
    let mut baselines = BTreeMap::new();
    for i in 0..n {
        let path = RemotePath::parse(&format!("/bench/dir{}/file{i}", i / 1000)).expect("path");
        let bytes = (i as u64).to_le_bytes();
        let v = Version::new(8, i as i64, Some(Hash::of(&bytes)));
        local.insert(path.clone(), (EntryKind::File, v.clone()));
        // Half the paths changed remotely.
        let r = if i % 2 == 0 {
            v.clone()
        } else {
            Version::new(9, i as i64, Some(Hash::of(b"other")))
        };
        remote.insert(path.clone(), (EntryKind::File, r.clone()));
        baselines.insert(
            path.clone(),
            BaselineEntry {
                path,
                kind: EntryKind::File,
                local: v.clone(),
                remote: v,
                synced_at_ms: 0,
            },
        );
    }
    let start = Instant::now();
    let actions = plan(&local, &remote, &baselines, 0);
    let elapsed = start.elapsed();
    let per = elapsed.as_secs_f64() / n as f64 * 1e6;
    println!(
        "reconciler {n} paths in {elapsed:?} ({per:.2} µs/path), {} actions",
        actions.len()
    );
    assert_eq!(actions.len(), n / 2);
}
