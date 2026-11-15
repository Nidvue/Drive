// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! End-to-end sync tests against an in-memory remote.
//!
//! These are the correctness invariants that matter: no silent loss, a
//! conflict always produces a copy, and the journal records intent before the
//! mutation is attempted.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use nidvue_cache::Index;
use nidvue_core::config::{SyncConfig, SyncPair};
use nidvue_core::error::{NidvueError, Result};
use nidvue_core::path::RemotePath;
use nidvue_core::time::now_ms;
use nidvue_core::types::EntryKind;
use nidvue_jotta::xml::{ListStream, RemoteEntry, RemoteFile, RemoteFolder, Stats};
use nidvue_sync::SyncEngine;
use nidvue_vfs::Remote;

#[derive(Default)]
struct FakeRemote {
    files: Mutex<BTreeMap<String, (Vec<u8>, i64)>>,
    dirs: Mutex<BTreeSet<String>>,
    fail_upload: AtomicBool,
}

impl FakeRemote {
    fn put(&self, path: &str, bytes: &[u8]) {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_string(), (bytes.to_vec(), now_ms()));
        self.add_parents(path);
    }

    fn add_parents(&self, path: &str) {
        let mut dirs = self.dirs.lock().unwrap();
        let mut current = String::new();
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            if !current.is_empty() {
                dirs.insert(current.clone());
            }
            if current.is_empty() {
                current.push_str(seg);
            } else {
                current.push('/');
                current.push_str(seg);
            }
        }
        // The file itself is not a directory.
        dirs.remove(path);
    }

    fn get(&self, path: &str) -> Option<Vec<u8>> {
        self.files.lock().unwrap().get(path).map(|(b, _)| b.clone())
    }

    fn names(&self) -> Vec<String> {
        self.files.lock().unwrap().keys().cloned().collect()
    }
}

#[async_trait]
impl Remote for FakeRemote {
    async fn remote_stat(&self, path: &RemotePath) -> Result<RemoteEntry> {
        let key = path.as_str().trim_start_matches('/').to_string();
        let files = self.files.lock().unwrap();
        if let Some((bytes, mtime)) = files.get(&key) {
            return Ok(RemoteEntry {
                kind: EntryKind::File,
                name: key.rsplit('/').next().unwrap_or(&key).to_string(),
                path: key,
                size: bytes.len() as u64,
                md5: Some(nidvue_core::hash::md5_reader(bytes.as_slice())?),
                state: Some("COMPLETED".into()),
                modified_ms: *mtime,
                created_ms: *mtime,
            });
        }
        let dirs = self.dirs.lock().unwrap();
        if key.is_empty() || dirs.contains(&key) {
            return Ok(RemoteEntry {
                kind: EntryKind::Dir,
                name: key.rsplit('/').next().unwrap_or("").to_string(),
                path: key,
                size: 0,
                md5: None,
                state: None,
                modified_ms: 0,
                created_ms: 0,
            });
        }
        Err(NidvueError::NotFound(path.to_string()))
    }

    async fn remote_list(&self, path: &RemotePath) -> Result<ListStream> {
        let base = path.as_str().trim_start_matches('/').to_string();
        let mut out = ListStream::default();
        let dirs = self.dirs.lock().unwrap().clone();
        for dir in dirs.iter().filter(|d| d.starts_with(&base) && **d != base) {
            out.folders.push(RemoteFolder { path: dir.clone() });
        }
        let files = self.files.lock().unwrap();
        for (key, (bytes, mtime)) in files.iter().filter(|(k, _)| k.starts_with(&base)) {
            out.files.push(RemoteFile {
                path: key
                    .rsplit_once('/')
                    .map(|(d, _)| d.to_string())
                    .unwrap_or_default(),
                name: key.rsplit('/').next().unwrap_or(key).to_string(),
                size: bytes.len() as u64,
                md5: nidvue_core::hash::md5_reader(bytes.as_slice()).ok(),
                modified_ms: *mtime,
                created_ms: *mtime,
            });
        }
        out.stats = Some(Stats {
            folders: out.folders.len() as u64,
            files: out.files.len() as u64,
        });
        Ok(out)
    }

    async fn remote_download(
        &self,
        path: &RemotePath,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _expected_size: Option<u64>,
    ) -> Result<u64> {
        use tokio::io::AsyncWriteExt;
        let key = path.as_str().trim_start_matches('/').to_string();
        let bytes = self
            .files
            .lock()
            .unwrap()
            .get(&key)
            .map(|(b, _)| b.clone())
            .ok_or_else(|| NidvueError::NotFound(path.to_string()))?;
        writer
            .write_all(&bytes)
            .await
            .map_err(|e| NidvueError::io(path.as_str(), e))?;
        Ok(bytes.len() as u64)
    }

    async fn remote_upload(&self, path: &RemotePath, local: &Path) -> Result<RemoteEntry> {
        if self.fail_upload.load(Ordering::SeqCst) {
            return Err(NidvueError::Network("upload disabled for this test".into()));
        }
        let bytes = std::fs::read(local).map_err(|e| NidvueError::io(local, e))?;
        let key = path.as_str().trim_start_matches('/').to_string();
        // One write is one version: the timestamp this call reports must be the
        // timestamp it stored, or the next listing looks like a remote change and
        // the engine re-downloads a file nobody touched. Two separate `now_ms()`
        // calls differ by a millisecond often enough to fail in CI and never here.
        let mtime = now_ms();
        self.files
            .lock()
            .unwrap()
            .insert(key.clone(), (bytes.clone(), mtime));
        self.add_parents(&key);
        Ok(RemoteEntry {
            kind: EntryKind::File,
            name: key.rsplit('/').next().unwrap_or(&key).to_string(),
            path: key,
            size: bytes.len() as u64,
            md5: Some(nidvue_core::hash::md5_reader(bytes.as_slice())?),
            state: Some("COMPLETED".into()),
            modified_ms: mtime,
            created_ms: mtime,
        })
    }

    async fn remote_mkdir(&self, path: &RemotePath) -> Result<()> {
        let key = path.as_str().trim_start_matches('/').to_string();
        if !key.is_empty() {
            self.dirs.lock().unwrap().insert(key);
        }
        Ok(())
    }

    async fn remote_delete(&self, path: &RemotePath, _permanent: bool) -> Result<()> {
        let key = path.as_str().trim_start_matches('/').to_string();
        let removed = self.files.lock().unwrap().remove(&key).is_some();
        self.dirs.lock().unwrap().remove(&key);
        let _ = removed;
        Ok(())
    }

    async fn remote_rename(&self, from: &RemotePath, to: &RemotePath) -> Result<()> {
        let a = from.as_str().trim_start_matches('/').to_string();
        let b = to.as_str().trim_start_matches('/').to_string();
        let mut files = self.files.lock().unwrap();
        let entry = files
            .remove(&a)
            .ok_or_else(|| NidvueError::NotFound(from.to_string()))?;
        files.insert(b, entry);
        Ok(())
    }
}

struct Harness {
    engine: SyncEngine,
    remote: Arc<FakeRemote>,
    index: Arc<Index>,
    local: tempfile::TempDir,
    _trash: tempfile::TempDir,
}

fn harness() -> Harness {
    let remote = Arc::new(FakeRemote::default());
    let index = Arc::new(Index::in_memory().unwrap());
    let (tx, _rx) = tokio::sync::broadcast::channel(64);
    let trash = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    let cfg = SyncConfig {
        workers: 1,
        ..Default::default()
    };
    let engine = SyncEngine::new(
        remote.clone(),
        index.clone(),
        cfg,
        trash.path().to_path_buf(),
        tx,
    );
    Harness {
        engine,
        remote,
        index,
        local,
        _trash: trash,
    }
}

fn pair(local: &Path) -> SyncPair {
    SyncPair {
        name: "test".into(),
        local: local.to_path_buf(),
        remote: "/Docs".into(),
        paused: false,
        include: vec![],
        exclude: vec![],
        conflict_policy: None,
    }
}

fn write_local(root: &Path, rel: &str, bytes: &[u8]) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

#[tokio::test]
async fn local_files_are_uploaded_and_baselined() {
    let h = harness();
    write_local(h.local.path(), "a.txt", b"hello");
    write_local(h.local.path(), "sub/b.txt", b"world");
    let report = h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    assert_eq!(report.applied, 3, "dir + two files: {report:?}");
    assert_eq!(h.remote.get("Docs/a.txt").as_deref(), Some(&b"hello"[..]));
    assert_eq!(
        h.remote.get("Docs/sub/b.txt").as_deref(),
        Some(&b"world"[..])
    );
    assert_eq!(h.index.all_baselines().unwrap().len(), 3);
    // Nothing is left in the journal.
    assert!(h.index.journal_pending().unwrap().is_empty());
}

#[tokio::test]
async fn remote_files_are_downloaded_atomically() {
    let h = harness();
    h.remote.put("Docs/r.txt", b"remote bytes");
    let report = h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    assert_eq!(report.applied, 1, "{report:?}");
    let got = std::fs::read(h.local.path().join("r.txt")).unwrap();
    assert_eq!(got, b"remote bytes");
    // No temp file survives.
    let leftovers: Vec<PathBuf> = std::fs::read_dir(h.local.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().contains("nidvue-tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[tokio::test]
async fn a_conflict_keeps_both_versions_and_records_it() {
    let h = harness();
    write_local(h.local.path(), "a.txt", b"base");
    h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    // Both sides change, differently.
    write_local(h.local.path(), "a.txt", b"local edit");
    h.remote.put("Docs/a.txt", b"remote edit");
    let report = h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    assert_eq!(report.conflicts, 1, "{report:?}");
    assert!(
        report.failed.is_empty(),
        "conflict action failed: {report:?}"
    );

    let local_names: Vec<String> = std::fs::read_dir(h.local.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        local_names.iter().any(|n| n.contains("(conflict Nidvue ")),
        "{local_names:?}"
    );
    // Both versions are on disk, and on the remote.
    let all_local: Vec<Vec<u8>> = local_names
        .iter()
        .filter(|n| n.ends_with(".txt"))
        .map(|n| std::fs::read(h.local.path().join(n)).unwrap())
        .collect();
    assert!(all_local.contains(&b"local edit".to_vec()), "{all_local:?}");
    assert!(
        all_local.contains(&b"remote edit".to_vec()),
        "{all_local:?}"
    );
    let remote_names = h.remote.names();
    assert!(
        remote_names.iter().any(|n| n.contains("(conflict Nidvue ")),
        "{remote_names:?}"
    );
    assert_eq!(h.index.conflicts(true).unwrap().len(), 1);
}

#[tokio::test]
async fn a_failed_upload_leaves_a_journal_row_and_the_file() {
    let h = harness();
    h.remote.fail_upload.store(true, Ordering::SeqCst);
    write_local(h.local.path(), "a.txt", b"important");
    let report = h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    assert_eq!(report.applied, 0);
    assert_eq!(report.failed.len(), 1);
    let pending = h.index.journal_pending().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].op, "upload");
    // The local file is untouched: a failed upload never destroys data.
    assert_eq!(
        std::fs::read(h.local.path().join("a.txt")).unwrap(),
        b"important"
    );
}

#[tokio::test]
async fn recovery_commits_only_what_it_can_observe() {
    let h = harness();
    // An intent whose effect already happened (crash after the upload, before
    // the commit) is committed.
    h.remote.put("Docs/done.txt", b"x");
    let done = h
        .index
        .journal_append("op-1", "/Docs/done.txt", "upload", None, Some(1), None)
        .unwrap();
    // An intent whose effect did not happen stays pending.
    let missing = h
        .index
        .journal_append("op-2", "/Docs/missing.txt", "upload", None, Some(1), None)
        .unwrap();
    let report = h.engine.recover().await.unwrap();
    assert_eq!(report.examined, 2);
    assert_eq!(report.committed, 1);
    assert_eq!(report.pending.len(), 1);
    let pending = h.index.journal_pending().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, missing);
    assert!(!pending.iter().any(|r| r.id == done));
}

#[tokio::test]
async fn local_deletion_propagates_remotely() {
    let h = harness();
    write_local(h.local.path(), "gone.txt", b"bye");
    h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    assert!(h.remote.get("Docs/gone.txt").is_some());
    std::fs::remove_file(h.local.path().join("gone.txt")).unwrap();
    let report = h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    assert_eq!(report.applied, 1, "{report:?}");
    assert!(h.remote.get("Docs/gone.txt").is_none());
}

#[tokio::test]
async fn remote_deletion_moves_the_local_file_to_trash() {
    let h = harness();
    h.remote.put("Docs/r.txt", b"data");
    h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    assert!(h.local.path().join("r.txt").exists());
    h.remote.files.lock().unwrap().remove("Docs/r.txt");
    let report = h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    assert_eq!(report.applied, 1, "{report:?}");
    assert!(!h.local.path().join("r.txt").exists());
    // The trash keeps a copy for the retention window.
    let trash_entries = walk(h._trash.path());
    assert!(
        trash_entries.iter().any(|p| p.ends_with("r.txt")),
        "{trash_entries:?}"
    );
}

#[tokio::test]
async fn a_second_pass_with_no_changes_plans_nothing() {
    let h = harness();
    write_local(h.local.path(), "a.txt", b"stable");
    h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    let report = h.engine.sync_pair(&pair(h.local.path())).await.unwrap();
    assert_eq!(report.planned, 0, "{report:?}");
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out
}
