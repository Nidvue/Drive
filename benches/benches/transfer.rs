// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Cold read and sustained transfer against an emulated link.
//!
//! The remote here is synthetic on purpose: it serves bytes at a fixed rate, so
//! "sustained throughput as a fraction of link capacity" is a number this bench
//! can actually produce without a real 50 Mbit line. The rate is the bench's
//! own input, printed alongside the result, never a claim about Jottacloud.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nidvue_cache::Cache;
use nidvue_core::config::CacheConfig;
use nidvue_core::error::Result;
use nidvue_core::path::RemotePath;
use nidvue_jotta::xml::{ListStream, RemoteEntry};
use nidvue_vfs::{EngineVfs, ProgressFn, Remote, Vfs};

const LINK_BYTES_PER_SECOND: u64 = 6_250_000;
const CHUNK: u64 = 64 * 1024;

/// A remote that serves `size` bytes at `bytes_per_second`.
struct Link {
    size: u64,
    bytes_per_second: u64,
}

impl Link {
    fn chunk_delay(&self) -> Duration {
        let per_second = self.bytes_per_second.max(1);
        Duration::from_secs_f64(CHUNK as f64 / per_second as f64)
    }
}

#[async_trait::async_trait]
impl Remote for Link {
    async fn remote_stat(&self, path: &RemotePath) -> Result<RemoteEntry> {
        Ok(RemoteEntry {
            kind: nidvue_core::types::EntryKind::File,
            name: path.file_name().unwrap_or_default().to_string(),
            path: path.as_str().trim_start_matches('/').to_string(),
            size: self.size,
            md5: None,
            state: Some("COMPLETED".into()),
            modified_ms: 0,
            created_ms: 0,
        })
    }

    async fn remote_list(&self, _path: &RemotePath) -> Result<ListStream> {
        Ok(ListStream::default())
    }

    async fn remote_download(
        &self,
        _path: &RemotePath,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _expected_size: Option<u64>,
        progress: Option<ProgressFn>,
    ) -> Result<u64> {
        use tokio::io::AsyncWriteExt;
        let payload = vec![0x5au8; CHUNK as usize];
        let mut written = 0u64;
        while written < self.size {
            let take = CHUNK.min(self.size - written) as usize;
            writer
                .write_all(&payload[..take])
                .await
                .map_err(|e| nidvue_core::NidvueError::Internal(format!("synthetic write: {e}")))?;
            written += take as u64;
            if let Some(report) = &progress {
                report(written, self.size);
            }
            tokio::time::sleep(self.chunk_delay()).await;
        }
        Ok(written)
    }

    async fn remote_upload(
        &self,
        path: &RemotePath,
        _local: &Path,
        progress: Option<ProgressFn>,
    ) -> Result<RemoteEntry> {
        if let Some(report) = &progress {
            report(self.size, self.size);
        }
        self.remote_stat(path).await
    }

    async fn remote_mkdir(&self, _path: &RemotePath) -> Result<()> {
        Ok(())
    }

    async fn remote_delete(&self, _path: &RemotePath, _permanent: bool) -> Result<()> {
        Ok(())
    }

    async fn remote_rename(&self, _from: &RemotePath, _to: &RemotePath) -> Result<()> {
        Ok(())
    }
}

fn cache_at(dir: &Path) -> Cache {
    let config = CacheConfig {
        max_size_bytes: 512 * 1024 * 1024,
        ..Default::default()
    };
    Cache::open(&config, &dir.join("blobs"), &dir.join("index.sqlite")).expect("cache")
}

/// Peak resident set in MiB, from `/proc/self/status` on Linux.
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

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async_main());
}

async fn async_main() {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("nidvue-bench-transfer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("bench dir");
    let cache = Arc::new(cache_at(&dir));

    let link_mbit = LINK_BYTES_PER_SECOND as f64 * 8.0 / 1e6;

    // Cold read: a 1 MiB file on a 50 Mbit/s link. The engine fetches the whole
    // object atomically, so the read costs one file, not one read call.
    let cold_link = Arc::new(Link {
        size: 1024 * 1024,
        bytes_per_second: LINK_BYTES_PER_SECOND,
    });
    let cold_vfs = EngineVfs::new(cold_link, cache.clone(), dir.join("spool-cold")).expect("vfs");
    let cold_path = RemotePath::parse("/cold.bin").expect("path");
    let cold_start = Instant::now();
    let cold = cold_vfs
        .read(&cold_path, 0, 1024 * 1024)
        .await
        .expect("cold read");
    let cold_elapsed = cold_start.elapsed();
    assert_eq!(cold.len(), 1024 * 1024);
    println!(
        "cold read 1 MiB over {link_mbit:.1} Mbit/s: {:?} (budget ≤ 400 ms) {}",
        cold_elapsed,
        if cold_elapsed <= Duration::from_millis(400) {
            "PASS"
        } else {
            "MISS"
        }
    );

    // Sustained: 64 distinct 128 KiB objects, so every byte crosses the link and
    // the per-object overhead is inside the measurement.
    const FILES: usize = 64;
    const FILE_SIZE: u64 = 128 * 1024;
    let sustained_link = Arc::new(Link {
        size: FILE_SIZE,
        bytes_per_second: LINK_BYTES_PER_SECOND,
    });
    let sustained_vfs =
        EngineVfs::new(sustained_link, cache.clone(), dir.join("spool-sustained")).expect("vfs");
    let sustained_start = Instant::now();
    let mut total = 0u64;
    for i in 0..FILES {
        let path = RemotePath::parse(&format!("/sustained/f{i:03}.bin")).expect("path");
        let bytes = sustained_vfs
            .read(&path, 0, FILE_SIZE as usize)
            .await
            .expect("sustained read");
        total += bytes.len() as u64;
    }
    let sustained = sustained_start.elapsed();
    let achieved = total as f64 / sustained.as_secs_f64();
    let fraction = achieved / LINK_BYTES_PER_SECOND as f64;
    println!(
        "sustained transfer {:.2} MiB in {FILES} objects in {:?} over {link_mbit:.1} Mbit/s: {:.2} MiB/s = {:.1} % of the link (budget ≥ 80 %) {}",
        total as f64 / (1024.0 * 1024.0),
        sustained,
        achieved / (1024.0 * 1024.0),
        fraction * 100.0,
        if fraction >= 0.80 { "PASS" } else { "MISS" }
    );

    let warm_start = Instant::now();
    let warm = cold_vfs
        .read(&cold_path, 0, 1024 * 1024)
        .await
        .expect("warm read");
    let warm_elapsed = warm_start.elapsed();
    assert_eq!(warm.len(), 1024 * 1024);
    let speedup = cold_elapsed.as_secs_f64() / warm_elapsed.as_secs_f64().max(1e-9);
    println!(
        "warm read 1 MiB: {:?} ({:.0}× the cold read, budget ≥ 100×) {}",
        warm_elapsed,
        speedup,
        if speedup >= 100.0 { "PASS" } else { "MISS" }
    );

    println!("peak rss: {:.1} MiB", peak_rss_mib());
    drop(cold_vfs);
    drop(sustained_vfs);
    drop(cache);
    let _ = std::fs::remove_dir_all(&dir);
}
