// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! A 100k-path initial sync: local scan, index write, three-way plan.
//!
//! What this measures is the part that is bounded by the engine: walking 100k
//! small files, writing their metadata, and planning the first pass against an
//! empty remote. It is not 100k network transfers, and it says so: the transfer
//! half is bounded by the link, not by this code.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use nidvue_cache::{Cache, EntryRow, Index};
use nidvue_core::config::CacheConfig;
use nidvue_core::path::RemotePath;
use nidvue_core::types::EntryKind;
use nidvue_sync::{plan, scan_local, PathFilter};

const PATHS: usize = 100_000;
const FILES_PER_DIR: usize = 1000;

fn peak_rss_mib() -> f64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return 0.0;
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let kb: f64 = rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse()
                .unwrap_or(0.0);
            return kb / 1024.0;
        }
    }
    0.0
}

fn build_tree(root: &Path) -> std::time::Duration {
    let start = Instant::now();
    for i in 0..PATHS {
        let dir = root.join(format!("dir{:03}", i / FILES_PER_DIR));
        if i % FILES_PER_DIR == 0 {
            std::fs::create_dir_all(&dir).expect("dir");
        }
        std::fs::write(dir.join(format!("f{i:06}.txt")), b"x").expect("file");
    }
    start.elapsed()
}

fn main() {
    let root = std::env::temp_dir().join(format!("nidvue-bench-sync-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");

    let tree_time = build_tree(&root);
    println!("built {PATHS} files in {tree_time:?} (fixture, not engine work)");

    let filter = PathFilter::new(&[], &[]).expect("filter");
    let start = Instant::now();
    let local = scan_local(&root, &RemotePath::root(), &filter, 0).expect("scan");
    let scan_time = start.elapsed();
    let expected = PATHS + PATHS / FILES_PER_DIR;
    assert_eq!(local.len(), expected, "scanned {} paths", local.len());
    println!(
        "scan {PATHS} paths: {scan_time:?} ({:.1} µs/path)",
        scan_time.as_secs_f64() / PATHS as f64 * 1e6
    );

    let cache_dir = root.join(".bench-cache");
    let cache = Cache::open(
        &CacheConfig::default(),
        &cache_dir.join("blobs"),
        &cache_dir.join("index.sqlite"),
    )
    .expect("cache");
    let index: &Index = cache.index();
    let start = Instant::now();
    for (path, (kind, version)) in &local {
        index
            .upsert_entry(&EntryRow {
                path: path.as_str().to_string(),
                kind: *kind,
                size: version.size,
                mtime_ms: version.mtime_ms,
                hash: version.hash.as_ref().map(|h| h.as_str().to_string()),
                remote_rev: None,
                dirty: false,
            })
            .expect("index row");
    }
    let index_time = start.elapsed();
    println!(
        "index {PATHS} rows: {index_time:?} ({:.1} µs/row, {} entries stored)",
        index_time.as_secs_f64() / PATHS as f64 * 1e6,
        index.count_entries().expect("count")
    );

    let remote: BTreeMap<RemotePath, (EntryKind, nidvue_core::types::Version)> = BTreeMap::new();
    let baselines = BTreeMap::new();
    let start = Instant::now();
    let actions = plan(&local, &remote, &baselines);
    let plan_time = start.elapsed();
    assert_eq!(actions.len(), expected);
    println!(
        "first plan over an empty remote: {plan_time:?} ({:.2} µs/path, {} actions)",
        plan_time.as_secs_f64() / PATHS as f64 * 1e6,
        actions.len()
    );

    let peak = peak_rss_mib();
    println!(
        "peak rss: {peak:.1} MiB for {PATHS} paths ({:.0} bytes/path)",
        peak * 1024.0 * 1024.0 / PATHS as f64
    );
    let total = scan_time + index_time + plan_time;
    println!("engine total: {total:?}");

    let _ = std::fs::remove_dir_all(&root);
}
