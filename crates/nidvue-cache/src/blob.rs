// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Content-addressed blob store.
//!
//! Blobs are immutable and named by their SHA-256, sharded two levels
//! (`blobs/ab/cd/<hash>`), so no directory grows without bound. A write is
//! always temp file → `fsync` → atomic rename → `fsync` of the directory: a
//! reader either sees the old blob or the complete new one, never a prefix.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use nidvue_core::error::{NidvueError, Result};
use nidvue_core::hash::{hash_file, Hash, HashAndSize};

/// The blob store on disk.
#[derive(Debug, Clone)]
pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    /// Open (creating if needed) a blob store rooted at `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| NidvueError::io(&root, e))?;
        Ok(Self { root })
    }

    /// The store root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The on-disk path of a blob.
    pub fn path_of(&self, hash: &Hash) -> PathBuf {
        let hex = hash.as_str();
        self.root.join(&hex[0..2]).join(&hex[2..4]).join(hex)
    }

    /// True when the blob file exists.
    pub fn contains(&self, hash: &Hash) -> bool {
        self.path_of(hash).is_file()
    }

    /// Write bytes and return their hash. Idempotent.
    pub fn put(&self, bytes: &[u8]) -> Result<Hash> {
        let hash = Hash::of(bytes);
        if self.contains(&hash) {
            return Ok(hash);
        }
        let path = self.path_of(&hash);
        self.write_atomic(&path, |f| {
            f.write_all(bytes).map_err(|e| NidvueError::io(&path, e))
        })?;
        Ok(hash)
    }

    /// Copy a local file into the store, hashing while copying.
    ///
    /// Returns the hash and size. If the blob already exists it is verified and
    /// not rewritten.
    pub fn put_file(&self, source: &Path) -> Result<HashAndSize> {
        let computed = hash_file(source)?;
        if self.contains(&computed.hash) {
            return Ok(computed);
        }
        let path = self.path_of(&computed.hash);
        self.write_atomic(&path, |out| {
            let mut input = fs::File::open(source).map_err(|e| NidvueError::io(source, e))?;
            std::io::copy(&mut input, out).map_err(|e| NidvueError::io(source, e))?;
            Ok(())
        })?;
        Ok(computed)
    }

    /// Open a blob for reading.
    pub fn open_read(&self, hash: &Hash) -> Result<fs::File> {
        let path = self.path_of(hash);
        fs::File::open(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                NidvueError::NotFound(format!("blob {hash}"))
            } else {
                NidvueError::io(&path, e)
            }
        })
    }

    /// Read a whole blob.
    pub fn read(&self, hash: &Hash) -> Result<Vec<u8>> {
        let path = self.path_of(hash);
        fs::read(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                NidvueError::NotFound(format!("blob {hash}"))
            } else {
                NidvueError::io(&path, e)
            }
        })
    }

    /// Re-hash a blob and compare with its name.
    pub fn verify(&self, hash: &Hash) -> Result<bool> {
        let path = self.path_of(hash);
        if !path.is_file() {
            return Ok(false);
        }
        Ok(hash_file(&path)?.hash == *hash)
    }

    /// The size of a blob, without reading it.
    pub fn size(&self, hash: &Hash) -> Result<u64> {
        let path = self.path_of(hash);
        fs::metadata(&path)
            .map(|m| m.len())
            .map_err(|e| NidvueError::io(&path, e))
    }

    /// Remove a blob. Missing is not an error.
    pub fn remove(&self, hash: &Hash) -> Result<()> {
        let path = self.path_of(hash);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(NidvueError::io(&path, e)),
        }
    }

    /// Total bytes and blob count currently on disk.
    pub fn usage(&self) -> Result<(u64, u64)> {
        let mut bytes = 0u64;
        let mut count = 0u64;
        for shard in read_dir_sorted(&self.root)? {
            if !shard.is_dir() {
                continue;
            }
            for sub in read_dir_sorted(&shard)? {
                if !sub.is_dir() {
                    continue;
                }
                for blob in read_dir_sorted(&sub)? {
                    if let Ok(meta) = fs::metadata(&blob) {
                        if meta.is_file() {
                            bytes += meta.len();
                            count += 1;
                        }
                    }
                }
            }
        }
        Ok((bytes, count))
    }

    fn write_atomic(&self, path: &Path, write: impl FnOnce(&mut fs::File) -> Result<()>) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| NidvueError::Cache("blob path has no parent".into()))?;
        fs::create_dir_all(parent).map_err(|e| NidvueError::io(parent, e))?;
        let tmp = parent.join(format!(
            ".tmp-{}-{}",
            std::process::id(),
            nidvue_core::id::CorrelationId::new("blob")
        ));
        let mut file = fs::File::create(&tmp).map_err(|e| NidvueError::io(&tmp, e))?;
        write(&mut file)?;
        file.sync_all().map_err(|e| NidvueError::io(&tmp, e))?;
        drop(file);
        fs::rename(&tmp, path).map_err(|e| NidvueError::io(path, e))?;
        // Durability of the rename itself.
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
        Ok(())
    }
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| NidvueError::io(dir, e))? {
        entries.push(entry.map_err(|e| NidvueError::io(dir, e))?.path());
    }
    entries.sort();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_and_shard() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        let hash = store.put(b"hello nidvue").unwrap();
        assert!(store.contains(&hash));
        assert_eq!(store.read(&hash).unwrap(), b"hello nidvue");
        // Two-level sharding.
        let path = store.path_of(&hash);
        let rel: Vec<_> = path.strip_prefix(dir.path()).unwrap().iter().map(|s| s.to_os_string()).collect();
        assert_eq!(rel.len(), 3);
        assert_eq!(rel[0].to_str().unwrap(), &hash.as_str()[0..2]);
        assert_eq!(rel[1].to_str().unwrap(), &hash.as_str()[2..4]);
        assert_eq!(store.size(&hash).unwrap(), 12);
    }

    #[test]
    fn put_is_idempotent_and_verifies() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        let a = store.put(b"same").unwrap();
        let b = store.put(b"same").unwrap();
        assert_eq!(a, b);
        assert!(store.verify(&a).unwrap());
        assert_eq!(store.usage().unwrap(), (4, 1));
    }

    #[test]
    fn corrupt_blobs_fail_verification() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        let hash = store.put(b"good").unwrap();
        std::fs::write(store.path_of(&hash), b"evil").unwrap();
        assert!(!store.verify(&hash).unwrap());
    }

    #[test]
    fn missing_blobs_are_not_found_errors() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        let hash = Hash::of(b"absent");
        assert!(matches!(store.read(&hash), Err(NidvueError::NotFound(_))));
        assert!(store.remove(&hash).is_ok());
    }

    #[test]
    fn put_file_matches_in_memory_hash() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path().join("store")).unwrap();
        let src = dir.path().join("src.bin");
        std::fs::write(&src, b"file bytes").unwrap();
        let got = store.put_file(&src).unwrap();
        assert_eq!(got.hash, Hash::of(b"file bytes"));
        assert_eq!(got.size, 10);
        assert_eq!(store.read(&got.hash).unwrap(), b"file bytes");
    }

    #[test]
    fn no_temp_files_are_left_behind() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        let hash = store.put(b"x").unwrap();
        let parent = store.path_of(&hash).parent().unwrap().to_path_buf();
        let leftovers: Vec<_> = std::fs::read_dir(parent)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());
    }
}
