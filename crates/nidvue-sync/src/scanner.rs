// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Local and include/exclude scanning.
//!
//! The scan is synchronous and CPU-bound (hashing); callers run it inside
//! `spawn_blocking` so the async runtime keeps serving the mount.

use std::collections::BTreeMap;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use nidvue_core::error::{NidvueError, Result};
use nidvue_core::hash::hash_file;
use nidvue_core::path::{local_to_remote, RemotePath};
use nidvue_core::types::{EntryKind, Version};
use tracing::debug;

/// Include/exclude globs, matched against the path relative to the pair root.
#[derive(Debug, Clone, Default)]
pub struct PathFilter {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
}

impl PathFilter {
    /// Build a filter from include and exclude globs. Empty include = all.
    pub fn new(include: &[String], exclude: &[String]) -> Result<Self> {
        Ok(Self {
            include: build(include)?,
            exclude: build(exclude)?,
        })
    }

    /// True when a relative path is in scope.
    pub fn accepts(&self, relative: &str) -> bool {
        let included = match &self.include {
            Some(set) => set.is_match(relative),
            None => true,
        };
        if !included {
            return false;
        }
        match &self.exclude {
            Some(set) => !set.is_match(relative),
            None => true,
        }
    }
}

fn build(globs: &[String]) -> Result<Option<GlobSet>> {
    if globs.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for g in globs {
        builder.add(Glob::new(g).map_err(|e| NidvueError::Config(format!("bad glob {g:?}: {e}")))?);
    }
    Ok(Some(builder.build().map_err(|e| NidvueError::Config(format!("globset: {e}")))?))
}

/// Scan a local directory into remote-shaped entries.
///
/// Symlinks are skipped: a Jottacloud object is one path and a link target is
/// not representable, so pretending otherwise would lose data on the way back.
pub fn scan_local(
    root: &Path,
    base: &RemotePath,
    filter: &PathFilter,
    max_size_bytes: u64,
) -> Result<BTreeMap<RemotePath, (EntryKind, Version)>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(NidvueError::io(&dir, e)),
        };
        for entry in entries {
            let entry = entry.map_err(|e| NidvueError::io(&dir, e))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            // Our own temporary files are never user content.
            if name.ends_with(".nidvue-tmp") || name.starts_with(".nidvue-tmp-") {
                continue;
            }
            let meta = entry.metadata().map_err(|e| NidvueError::io(&path, e))?;
            if meta.file_type().is_symlink() {
                debug!(path = %path.display(), "skipping symlink");
                continue;
            }
            let relative_path = local_to_remote(root, &path)?;
            let remote = if base.is_root() {
                relative_path
            } else {
                RemotePath::parse(&format!(
                    "{}{}",
                    base.as_str().trim_end_matches('/'),
                    relative_path.as_str()
                ))?
            };
            let relative = remote.as_str().trim_start_matches('/');
            if meta.is_dir() {
                if !filter.accepts(relative) && !filter.accepts(&format!("{relative}/")) {
                    continue;
                }
                out.insert(
                    remote,
                    (
                        EntryKind::Dir,
                        Version::new(0, mtime_ms(&meta), None),
                    ),
                );
                stack.push(path);
            } else if meta.is_file() {
                if !filter.accepts(relative) {
                    continue;
                }
                if max_size_bytes > 0 && meta.len() > max_size_bytes {
                    debug!(path = %path.display(), size = meta.len(), "skipping oversize file");
                    continue;
                }
                let hashed = hash_file(&path)?;
                out.insert(
                    remote,
                    (
                        EntryKind::File,
                        Version::new(hashed.size, mtime_ms(&meta), Some(hashed.hash)),
                    ),
                );
            }
        }
    }
    Ok(out)
}

fn mtime_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    #[test]
    fn scans_files_and_directories() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.txt", b"a");
        write(dir.path(), "sub/b.txt", b"bb");
        let scan = scan_local(dir.path(), &RemotePath::root(), &PathFilter::default(), 0).unwrap();
        assert_eq!(scan.len(), 3);
        let (kind, version) = scan.get(&RemotePath::parse("/sub/b.txt").unwrap()).unwrap();
        assert_eq!(*kind, EntryKind::File);
        assert_eq!(version.size, 2);
        assert!(version.hash.is_some());
        assert!(scan.contains_key(&RemotePath::parse("/sub").unwrap()));
    }

    #[test]
    fn filters_include_and_exclude() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "keep.md", b"x");
        write(dir.path(), "skip.log", b"y");
        let filter = PathFilter::new(&["**/*.md".into()], &[]).unwrap();
        let scan = scan_local(dir.path(), &RemotePath::root(), &filter, 0).unwrap();
        let names: Vec<_> = scan.keys().map(|p| p.to_string()).collect();
        assert_eq!(names, vec!["/keep.md"]);

        let filter = PathFilter::new(&[], &["**/*.log".into()]).unwrap();
        let scan = scan_local(dir.path(), &RemotePath::root(), &filter, 0).unwrap();
        assert_eq!(scan.len(), 1);
    }

    #[test]
    fn skips_temp_and_oversize_files() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "big.bin", &[0u8; 32]);
        write(dir.path(), "x.nidvue-tmp", b"tmp");
        let scan = scan_local(dir.path(), &RemotePath::root(), &PathFilter::default(), 16).unwrap();
        assert!(scan.is_empty(), "{scan:?}");
    }

    #[test]
    fn rejects_bad_globs() {
        assert!(PathFilter::new(&["[".into()], &[]).is_err());
    }
}
