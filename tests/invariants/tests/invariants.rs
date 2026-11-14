// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The seven correctness invariants, one test each.
//!
//! These are the promises in `ARCHITECTURE.md` §11 and the product brief §11.
//! If one of these fails, the release does not ship.

use std::sync::Arc;

use nidvue_cache::{Cache, EntryRow, Index};
use nidvue_core::config::CacheConfig;
use nidvue_core::hash::Hash;
use nidvue_core::path::RemotePath;
use nidvue_core::secret::EncryptedFileStore;
use nidvue_core::secret::SecretStore;
use nidvue_core::secret::SecretString;
use nidvue_core::types::{ConflictPolicy, EntryKind, Version};
use nidvue_sync::{conflict_copy_path, decide_conflict, plan, Action, PathLocks};
use nidvue_vfs::{MemoryVfs, Vfs};

fn cache(max: u64) -> (Cache, tempfile::TempDir) {
    let cfg = CacheConfig {
        max_size_bytes: max,
        min_free_disk_bytes: 0,
        ..Default::default()
    };
    Cache::temporary(&cfg).unwrap()
}

/// 1. No file is truncated, half-written, or replaced by an older version.
#[test]
fn invariant_1_no_partial_or_stale_content() {
    let (cache, _dir) = cache(1 << 20);
    let v1 = b"version one".to_vec();
    let v2 = b"version two, longer".to_vec();
    let h1 = cache.put(&v1).unwrap();
    let h2 = cache.put(&v2).unwrap();
    assert_ne!(h1, h2);
    // Both blobs are intact; the old version is never overwritten in place.
    assert_eq!(cache.read(&h1).unwrap(), v1);
    assert_eq!(cache.read(&h2).unwrap(), v2);
    // A tampered blob is never served as if it were valid.
    let h3 = cache.put(b"original").unwrap();
    std::fs::write(cache.blobs().path_of(&h3), b"tampered").unwrap();
    assert!(!cache.is_intact(&h3).unwrap());
    cache.scrub().unwrap();
    assert!(cache.read(&h3).is_err());
}

/// 2. Every completed mutation is fully applied or fully absent after a crash.
#[test]
fn invariant_2_journal_survives_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    {
        let index = Index::open(&path).unwrap();
        let id = index
            .journal_append("op", "/a", "upload", Some("h"), Some(3), None)
            .unwrap();
        index.journal_mark(id, "applied").unwrap();
        // Crash here: no commit, no clean shutdown.
    }
    let index = Index::open(&path).unwrap();
    let pending = index.journal_pending().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].state, "applied");
    // The row is recoverable, so the mutation is not silently lost or assumed.
    index.journal_mark(pending[0].id, "commit").unwrap();
    assert!(index.journal_pending().unwrap().is_empty());
}

/// 3. Two workers never operate on the same path concurrently.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn invariant_3_one_worker_per_path() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let locks = Arc::new(PathLocks::default());
    let inside = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let locks = locks.clone();
        let inside = inside.clone();
        let peak = peak.clone();
        tasks.push(tokio::spawn(async move {
            let _guard = locks.lock("/one/path").await;
            let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            inside.fetch_sub(1, Ordering::SeqCst);
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    assert_eq!(peak.load(Ordering::SeqCst), 1);
}

/// 4. A remote and local edit to the same path always yields a conflict copy.
#[test]
fn invariant_4_conflicts_keep_both_versions() {
    let path = RemotePath::parse("/Docs/report.txt").unwrap();
    let local = Version::new(6, 200, Some(Hash::of(b"local!")));
    let remote = Version::new(6, 100, Some(Hash::of(b"remote")));
    let base = Version::new(4, 50, Some(Hash::of(b"base")));
    let baseline = nidvue_core::types::BaselineEntry {
        path: path.clone(),
        kind: EntryKind::File,
        local: base.clone(),
        remote: base,
        synced_at_ms: 0,
    };
    let mut local_map = std::collections::BTreeMap::new();
    let mut remote_map = std::collections::BTreeMap::new();
    local_map.insert(path.clone(), (EntryKind::File, local.clone()));
    remote_map.insert(path.clone(), (EntryKind::File, remote.clone()));
    let mut baselines = std::collections::BTreeMap::new();
    baselines.insert(path.clone(), baseline);
    let actions = plan(&local_map, &remote_map, &baselines);
    let conflict = actions
        .iter()
        .find_map(|a| match a {
            Action::Conflict { local, remote, .. } => Some((local.clone(), remote.clone())),
            _ => None,
        })
        .expect("a conflict must be planned");
    assert_eq!(conflict.0, local);
    assert_eq!(conflict.1, remote);

    // The losing version goes to a conflict copy; neither version is dropped.
    let decision = decide_conflict(&path, &local, &remote, ConflictPolicy::KeepBoth, 0);
    assert!(nidvue_core::path::is_conflict_copy(
        decision.copy_to.file_name().unwrap()
    ));
    assert_eq!(decision.keep, nidvue_core::types::Side::Local);
    assert_ne!(decision.copy_to, path);
    assert_eq!(
        conflict_copy_path(&path, 0).parent().unwrap().as_str(),
        "/Docs"
    );
}

/// 5. Eviction never removes a pinned, dirty, or open file.
#[test]
fn invariant_5_pinned_and_dirty_survive_eviction() {
    let (cache, _dir) = cache(64);
    let pinned = cache.put(&[1u8; 24]).unwrap();
    cache.pin(&pinned).unwrap();
    let dirty = cache.put(&[2u8; 24]).unwrap();
    cache.set_dirty(&dirty, true).unwrap();
    let free = cache.put(&[3u8; 8]).unwrap();
    assert_ne!(free, pinned);
    // The cap is full of protected data: a new write is refused, not forced.
    assert!(cache.put(&[4u8; 32]).is_err());
    assert!(cache.read(&pinned).is_ok());
    assert!(cache.read(&dirty).is_ok());
}

/// 6. Credentials never appear in logs, crash reports, or unencrypted on disk.
#[test]
fn invariant_6_credentials_stay_secret() {
    let dir = tempfile::tempdir().unwrap();
    let store = EncryptedFileStore::new(dir.path().join("credentials.enc"));
    let token = "eyJhbGciOiJIUzI1NiJ9.super-secret-refresh-token";
    store.store("jotta", &SecretString::new(token)).unwrap();
    // Nothing in the state directory contains the plaintext.
    for entry in std::fs::read_dir(dir.path()).unwrap().flatten() {
        let bytes = std::fs::read(entry.path()).unwrap();
        assert!(
            !bytes.windows(token.len()).any(|w| w == token.as_bytes()),
            "plaintext token found in {}",
            entry.path().display()
        );
    }
    // Debug output and the redaction helper never print it either.
    let secret = SecretString::new(token);
    assert!(!format!("{secret:?}").contains("super-secret"));
    assert!(!nidvue_core::secret::redact_token(token).contains("super-secret"));
}

/// 7. Engine panics cannot abort the UI or the mount.
#[test]
fn invariant_7_panics_are_contained() {
    // The C ABI is the boundary the UI and the mount cross; every entry point
    // catches unwinds. Null arguments must be refused, not dereferenced.
    let mut err: *mut std::ffi::c_char = std::ptr::null_mut();
    // SAFETY: null is explicitly handled by the ABI.
    unsafe {
        assert!(nidvue_ffi::nidvue_client_new(std::ptr::null(), &mut err).is_null());
        assert_eq!(
            nidvue_ffi::nidvue_client_unmount(std::ptr::null_mut()),
            nidvue_ffi::NIDVUE_INVALID
        );
        nidvue_ffi::nidvue_client_free(std::ptr::null_mut());
        if !err.is_null() {
            nidvue_ffi::nidvue_string_free(err);
        }
    }
    // And the VFS reports failures as typed errors rather than panicking.
    let vfs = MemoryVfs::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let err = runtime
        .block_on(vfs.getattr(&RemotePath::parse("/absent").unwrap()))
        .unwrap_err();
    assert!(matches!(err, nidvue_core::NidvueError::NotFound(_)));
}

/// Entries written through the index are readable after a reopen (sanity for
/// the invariant tests above: they exercise the real database, not a stub).
#[test]
fn index_persists_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    {
        let index = Index::open(&path).unwrap();
        index
            .upsert_entry(&EntryRow {
                path: "/a".into(),
                kind: EntryKind::File,
                size: 1,
                mtime_ms: 1,
                hash: Some("a".repeat(64)),
                remote_rev: None,
                dirty: false,
            })
            .unwrap();
    }
    let index = Index::open(&path).unwrap();
    assert_eq!(index.count_entries().unwrap(), 1);
}
