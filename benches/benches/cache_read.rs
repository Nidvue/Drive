// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Budget: warm cached read of 1 MiB ≤ 15 ms p50.

use std::time::{Duration, Instant};

use nidvue_cache::Cache;
use nidvue_core::config::CacheConfig;

fn p50(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() / 2]
}

fn main() {
    let cfg = CacheConfig { max_size_bytes: 256 * 1024 * 1024, min_free_disk_bytes: 0, ..Default::default() };
    let (cache, _dir) = Cache::temporary(&cfg).expect("cache");
    let payload = vec![0x5au8; 1024 * 1024];
    let hash = cache.put(&payload).expect("put");
    let mut samples = Vec::with_capacity(200);
    for _ in 0..200 {
        let start = Instant::now();
        let bytes = cache.read(&hash).expect("read");
        std::hint::black_box(&bytes);
        samples.push(start.elapsed());
    }
    let p50 = p50(samples);
    let budget = Duration::from_millis(15);
    println!("cache_read  1 MiB warm: p50 {p50:?} (budget ≤ {budget:?}) {}", verdict(p50 <= budget));
}

fn verdict(ok: bool) -> &'static str {
    if ok { "PASS" } else { "MISS" }
}
