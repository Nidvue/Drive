// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! A 100k-path initial sync: local scan, index write, the remote listing, and the
//! three-way plan against it.
//!
//! The remote is not empty any more: the bench builds the `liststream` document
//! the Jotta API returns for 100k entries and parses it with the production
//! parser, then plans local against remote. What is still not measured is the
//! HTTP transfer of those bytes or of the files: both are bounded by the link,
//! not by this code, and `docs/performance.md` says so.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use nidvue_cache::{Cache, EntryRow, Index};
use nidvue_core::config::CacheConfig;
use nidvue_core::path::RemotePath;
use nidvue_core::types::EntryKind;
use nidvue_jotta::xml::parse_list_stream;
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
    let rows: Vec<EntryRow> = local
        .iter()
        .map(|(path, (kind, version))| EntryRow {
            path: path.as_str().to_string(),
            kind: *kind,
            size: version.size,
            mtime_ms: version.mtime_ms,
            hash: version.hash.as_ref().map(|h| h.as_str().to_string()),
            remote_rev: None,
            dirty: false,
        })
        .collect();
    let start = Instant::now();
    index.upsert_entries(&rows).expect("index rows");
    let index_time = start.elapsed();
    println!(
        "index {PATHS} rows: {index_time:?} ({:.1} µs/row, {} entries stored)",
        index_time.as_secs_f64() / PATHS as f64 * 1e6,
        index.count_entries().expect("count")
    );

    // The remote listing as the service returns it: one `liststream` document
    // with every entry, parsed by the production streaming parser. Every other
    // entry is given a different hash, so the plan has real work to do instead of
    // walking a map of identical versions.
    let listing = remote_listing(&local);
    println!(
        "remote listing: {:.1} MiB of XML",
        listing.len() as f64 / (1024.0 * 1024.0)
    );
    let start = Instant::now();
    let stream = parse_list_stream(&listing).expect("the listing parses");
    let parse_time = start.elapsed();
    println!(
        "parse {PATHS} remote entries: {parse_time:?} ({:.2} µs/entry, {} folders, {} files)",
        parse_time.as_secs_f64() / PATHS as f64 * 1e6,
        stream.folders.len(),
        stream.files.len()
    );

    let remote: BTreeMap<RemotePath, (EntryKind, nidvue_core::types::Version)> = stream
        .files
        .iter()
        .map(|file| {
            let path = RemotePath::parse(&format!("/Archive/{}", file.path)).expect("remote path");
            let version = nidvue_core::types::Version {
                size: file.size,
                mtime_ms: file.modified_ms,
                hash: file
                    .md5
                    .as_ref()
                    .and_then(|md5| nidvue_core::Hash::from_hex(md5.clone()).ok()),
            };
            (path, (EntryKind::File, version))
        })
        .collect();
    assert_eq!(remote.len(), PATHS, "the listing must carry every path");

    let baselines = BTreeMap::new();
    let start = Instant::now();
    let actions = plan(&local, &remote, &baselines, 0);
    let plan_time = start.elapsed();
    println!(
        "first plan against the parsed remote: {plan_time:?} ({:.2} µs/path, {} actions)",
        plan_time.as_secs_f64() / PATHS as f64 * 1e6,
        actions.len()
    );
    assert!(
        !actions.is_empty(),
        "planning against a real remote listing must produce work"
    );

    let peak = peak_rss_mib();
    println!(
        "peak rss: {peak:.1} MiB for {PATHS} paths ({:.0} bytes/path)",
        peak * 1024.0 * 1024.0 / PATHS as f64
    );
    let total = scan_time + index_time + parse_time + plan_time;
    println!("engine total: {total:?}");

    let _ = std::fs::remove_dir_all(&root);
}

/// The `liststream` document the service returns for the local tree, with every
/// entry's md5 replaced so the remote differs from the local side.
fn remote_listing(
    local: &BTreeMap<RemotePath, (EntryKind, nidvue_core::types::Version)>,
) -> String {
    let mut files = String::with_capacity(local.len() * 160);
    let mut dirs = String::with_capacity(local.len() / FILES_PER_DIR * 60);
    for (path, (kind, version)) in local {
        let relative = path
            .as_str()
            .trim_start_matches('/')
            .trim_start_matches("Archive/");
        match kind {
            EntryKind::Dir => dirs.push_str(&format!("    <folder path=\"{relative}\"/>\n")),
            _ => {
                let md5 = "0F343B0931126A20F133D67C2B018A3B";
                files.push_str(&format!(
                    "    <file path=\"{relative}\" filename=\"{}\" md5=\"{md5}\" size=\"{}\" modified=\"2026-11-17T10:00:00.000Z\"/>\n",
                    relative.rsplit('/').next().unwrap_or(relative),
                    version.size,
                ));
            }
        }
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<folder time=\"2026-11-17-T10:00:00Z\" host=\"bench\">\n  <path xml:space=\"preserve\">/Jotta/Archive</path>\n  <folders>\n{dirs}  </folders>\n  <files>\n{files}  </files>\n  <stats folders=\"{folders}\" files=\"{count}\"/>\n</folder>\n",
        folders = local.len() / FILES_PER_DIR,
        count = local.len(),
    )
}
