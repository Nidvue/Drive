// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Mount an in-memory filesystem for a few seconds: a manual smoke test.
//!
//! ```sh
//! cargo run -p nidvue-mount --example mount_memory -- /tmp/nidvue-demo
//! ls /tmp/nidvue-demo
//! ```

use std::sync::Arc;

use nidvue_core::path::RemotePath;
use nidvue_core::types::EntryKind;
use nidvue_mount::{mount, MountOptions};
use nidvue_vfs::{MemoryVfs, Vfs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mountpoint = std::env::args().nth(1).unwrap_or_else(|| "/tmp/nidvue-demo".into());
    let runtime = tokio::runtime::Runtime::new()?;
    let _guard = runtime.enter();
    let vfs = Arc::new(MemoryVfs::new());
    runtime.block_on(async {
        vfs.create(&RemotePath::parse("/existing.txt")?, EntryKind::File).await?;
        vfs.write(&RemotePath::parse("/existing.txt")?, 0, b"seed").await?;
        Ok::<_, nidvue_core::NidvueError>(())
    })?;
    let handle = mount(vfs.clone(), std::path::Path::new(&mountpoint), MountOptions::default())?;
    println!("mounted at {mountpoint}; unmounting in 15 s");
    std::thread::sleep(std::time::Duration::from_secs(15));
    drop(handle);
    Ok(())
}
