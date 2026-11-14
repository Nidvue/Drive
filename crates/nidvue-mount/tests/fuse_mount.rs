// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! A real FUSE mount, exercised with ordinary filesystem calls.
//!
//! Skipped gracefully where the kernel, `/dev/fuse` or `fusermount3` is not
//! available, so the suite is green on a container without FUSE and in CI.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::sync::Arc;

use nidvue_core::path::RemotePath;
use nidvue_core::types::EntryKind;
use nidvue_mount::{mount, MountOptions};
use nidvue_vfs::{MemoryVfs, Vfs};

#[test]
#[ignore = "requires a real FUSE mount; run with --include-ignored"]
fn fuse_mount_serves_a_real_filesystem() {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return,
    };
    let _guard = runtime.enter();
    let vfs = Arc::new(MemoryVfs::new());
    runtime.block_on(async {
        vfs.create(
            &RemotePath::parse("/existing.txt").unwrap(),
            EntryKind::File,
        )
        .await
        .unwrap();
        vfs.write(&RemotePath::parse("/existing.txt").unwrap(), 0, b"seed")
            .await
            .unwrap();
    });

    let dir = tempfile::tempdir().unwrap();
    let mountpoint = dir.path().join("mnt");
    let mut handle = match mount(vfs.clone(), &mountpoint, MountOptions::default()) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipping: no FUSE here ({e})");
            return;
        }
    };

    // Read a file that existed before the mount.
    assert_eq!(
        std::fs::read(mountpoint.join("existing.txt")).unwrap(),
        b"seed"
    );

    // Create, write and read back.
    std::fs::write(mountpoint.join("hello.txt"), b"hello fuse").unwrap();
    assert_eq!(
        std::fs::read(mountpoint.join("hello.txt")).unwrap(),
        b"hello fuse"
    );

    // The listing shows both files.
    let mut names: Vec<String> = std::fs::read_dir(&mountpoint)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["existing.txt", "hello.txt"]);

    // Rename, then delete.
    std::fs::rename(mountpoint.join("hello.txt"), mountpoint.join("moved.txt")).unwrap();
    assert!(mountpoint.join("moved.txt").exists());
    std::fs::remove_file(mountpoint.join("moved.txt")).unwrap();
    assert!(!mountpoint.join("moved.txt").exists());

    // The engine saw the same mutations.
    let root = RemotePath::root();
    let entries = runtime.block_on(vfs.readdir(&root, 0, 10)).unwrap();
    assert_eq!(entries.len(), 1, "{entries:?}");

    handle.unmount();
}
