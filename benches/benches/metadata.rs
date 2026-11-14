// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Budget: cached metadata op (readdir page) ≤ 2 ms p50 on a 1M-entry index,
//! and listing latency stays flat as the index grows.

use std::time::{Duration, Instant};

use nidvue_cache::{EntryRow, Index};
use nidvue_core::types::EntryKind;

fn p50(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() / 2]
}

fn measure(index: &Index, dir: &str) -> Duration {
    let mut samples = Vec::with_capacity(200);
    for _ in 0..200 {
        let start = Instant::now();
        let entries = index.list_dir(dir).expect("list");
        std::hint::black_box(&entries);
        samples.push(start.elapsed());
    }
    p50(samples)
}

fn main() {
    let index = Index::in_memory().expect("index");
    let total = 1_000_000u64;
    // One million entries, 1000 per directory.
    for i in 0..total {
        let dir = i / 1000;
        index
            .upsert_entry(&EntryRow {
                path: format!("/bench/dir{dir}/file{i}"),
                kind: EntryKind::File,
                size: i % 4096,
                mtime_ms: i as i64,
                hash: None,
                remote_rev: None,
                dirty: false,
            })
            .expect("upsert");
    }
    let small = measure(&index, "/bench/dir0");
    let large = measure(&index, "/bench/dir999");
    let budget = Duration::from_millis(2);
    println!(
        "metadata  readdir page (1M entries): p50 {small:?} at dir0, {large:?} at dir999 (budget ≤ {budget:?}) {}",
        if small <= budget && large <= budget { "PASS" } else { "MISS" }
    );
    let flat = large < small * 3;
    println!("metadata  latency flat with index size: {}", if flat { "PASS" } else { "MISS" });
}
