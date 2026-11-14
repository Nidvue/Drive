// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Kernel filesystem adapters.
//!
//! One async [`Vfs`](nidvue_vfs::Vfs) trait, three thin adapters. Linux and
//! macOS share the FUSE3 implementation (`fuser`); Windows uses WinFSP through
//! the same trait and is compiled only on Windows.
//!
//! This crate holds no policy: no retries, no conflict logic, no caching
//! decisions. It translates kernel callbacks to trait calls and `NidvueError`
//! to `errno`. Kernel callbacks are synchronous, so each adapter owns a runtime
//! handle and dispatches to it; no callback holds a global lock.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nidvue_core::error::{NidvueError, Result};
use nidvue_core::path::RemotePath;
use nidvue_core::types::EntryKind;
use nidvue_vfs::{FsStats, Vfs};
use tracing::debug;

/// How the filesystem should be mounted.
#[derive(Debug, Clone)]
pub struct MountOptions {
    /// Mount read-only.
    pub read_only: bool,
    /// Pass `allow_other` to FUSE (requires `user_allow_other` in
    /// `/etc/fuse.conf`).
    pub allow_other: bool,
    /// Kernel attribute cache TTL.
    pub attr_ttl: Duration,
    /// Kernel entry cache TTL.
    pub entry_ttl: Duration,
    /// Filesystem name shown by `mount`.
    pub fs_name: String,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            read_only: false,
            allow_other: false,
            attr_ttl: Duration::from_millis(1_000),
            entry_ttl: Duration::from_millis(1_000),
            fs_name: "nidvue".into(),
        }
    }
}

/// Map a typed error to an `errno`. One function, one place to audit.
pub fn errno(error: &NidvueError) -> i32 {
    match error {
        NidvueError::NotFound(_) => libc::ENOENT,
        NidvueError::AlreadyExists(_) => libc::EEXIST,
        NidvueError::Auth(_) | NidvueError::Secret(_) => libc::EACCES,
        NidvueError::Conflict { detail, .. } if detail.contains("not empty") => libc::ENOTEMPTY,
        NidvueError::Conflict { .. } => libc::EEXIST,
        NidvueError::InvalidPath(_) => libc::EINVAL,
        NidvueError::Unsupported(_) => libc::EOPNOTSUPP,
        NidvueError::Cancelled => libc::EINTR,
        NidvueError::Mount(_) => libc::EIO,
        NidvueError::Cache(_) | NidvueError::Verification(_) => libc::EIO,
        NidvueError::RateLimited { .. } | NidvueError::Network(_) | NidvueError::Server { .. } => {
            libc::EIO
        }
        NidvueError::Io { source, .. } => match source.raw_os_error() {
            Some(code) => code,
            None => libc::EIO,
        },
        _ => libc::EIO,
    }
}

/// An inode table mapping kernel inodes to remote paths.
#[derive(Debug, Default)]
struct Inodes {
    by_ino: BTreeMap<u64, RemotePath>,
    by_path: BTreeMap<String, u64>,
    next: u64,
}

impl Inodes {
    fn new() -> Self {
        let mut table = Self { by_ino: BTreeMap::new(), by_path: BTreeMap::new(), next: 2 };
        table.by_ino.insert(1, RemotePath::root());
        table.by_path.insert("/".into(), 1);
        table
    }

    fn ino_for(&mut self, path: &RemotePath) -> u64 {
        if let Some(ino) = self.by_path.get(path.as_str()) {
            return *ino;
        }
        let ino = self.next;
        self.next += 1;
        self.by_ino.insert(ino, path.clone());
        self.by_path.insert(path.as_str().to_string(), ino);
        ino
    }

    fn path_of(&self, ino: u64) -> Option<RemotePath> {
        self.by_ino.get(&ino).cloned()
    }

    fn rename(&mut self, from: &RemotePath, to: &RemotePath) {
        let Some(ino) = self.by_path.remove(from.as_str()) else { return };
        self.by_ino.insert(ino, to.clone());
        self.by_path.insert(to.as_str().to_string(), ino);
        // Children of a renamed directory keep their inodes but move paths.
        let prefix = format!("{}/", from.as_str());
        let moved: Vec<(String, u64)> = self
            .by_path
            .iter()
            .filter(|(p, _)| p.starts_with(&prefix))
            .map(|(p, i)| (p.clone(), *i))
            .collect();
        for (old, ino) in moved {
            let suffix = old.trim_start_matches(from.as_str());
            let new_path = format!("{}{suffix}", to.as_str());
            self.by_path.remove(&old);
            self.by_path.insert(new_path.clone(), ino);
            if let Ok(parsed) = RemotePath::parse(&new_path) {
                self.by_ino.insert(ino, parsed);
            }
        }
    }

    fn forget(&mut self, ino: u64) {
        if let Some(path) = self.by_ino.remove(&ino) {
            self.by_path.remove(path.as_str());
        }
    }
}

fn now() -> SystemTime {
    SystemTime::now()
}

fn ms_to_time(ms: i64) -> SystemTime {
    if ms <= 0 {
        now()
    } else {
        UNIX_EPOCH + Duration::from_millis(ms as u64)
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod fuse {
    use super::*;
    use fuser::{
        FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty,
        ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite,
    };

    /// The FUSE3 filesystem.
    pub struct FuseVfs {
        vfs: Arc<dyn Vfs>,
        runtime: tokio::runtime::Handle,
        inodes: Inodes,
        options: MountOptions,
        uid: u32,
        gid: u32,
    }

    impl FuseVfs {
        /// Build the adapter. Must be called from inside a tokio runtime.
        pub fn new(vfs: Arc<dyn Vfs>, options: MountOptions) -> Result<Self> {
            let runtime = tokio::runtime::Handle::try_current().map_err(|_| {
                NidvueError::Mount("mount() must be called inside a tokio runtime context".into())
            })?;
            // SAFETY: `getuid`/`getgid` have no preconditions.
            let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
            Ok(Self { vfs, runtime, inodes: Inodes::new(), options, uid, gid })
        }

        fn attr(&mut self, ino: u64, path: &RemotePath) -> Option<FileAttr> {
            if path.is_root() {
                // The mount root always exists, whatever the engine says.
                let meta = nidvue_core::types::EntryMeta::dir(path.clone(), 0);
                return Some(self.attr_of(ino, path, &meta));
            }
            match self.runtime.block_on(self.vfs.getattr(path)) {
                Ok(meta) => Some(self.attr_of(ino, path, &meta)),
                Err(e) => {
                    debug!(path = %path, error = %e, "getattr failed");
                    None
                }
            }
        }

        fn attr_of(&self, ino: u64, _path: &RemotePath, meta: &nidvue_core::types::EntryMeta) -> FileAttr {
            let kind = match meta.kind {
                EntryKind::File => FileType::RegularFile,
                EntryKind::Dir => FileType::Directory,
                EntryKind::Symlink => FileType::Symlink,
            };
            let perm = match meta.kind {
                EntryKind::Dir => 0o755,
                _ => 0o644,
            };
            FileAttr {
                ino,
                size: meta.version.size,
                blocks: meta.version.size.div_ceil(512),
                atime: now(),
                mtime: ms_to_time(meta.version.mtime_ms),
                ctime: ms_to_time(meta.version.mtime_ms),
                crtime: ms_to_time(meta.version.mtime_ms),
                kind,
                perm,
                nlink: if meta.kind == EntryKind::Dir { 2 } else { 1 },
                uid: self.uid,
                gid: self.gid,
                rdev: 0,
                blksize: 4096,
                flags: 0,
            }
        }

        fn resolve(&mut self, parent: u64, name: &OsStr) -> Option<RemotePath> {
            let name = name.to_str()?;
            let parent = self.inodes.path_of(parent)?;
            if name == "." {
                return Some(parent);
            }
            if name == ".." {
                return Some(parent.parent().unwrap_or_else(RemotePath::root));
            }
            parent.join(name).ok()
        }
    }

    impl Filesystem for FuseVfs {
        fn lookup(&mut self, _req: &fuser::Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
            let Some(path) = self.resolve(parent, name) else {
                reply.error(libc::ENOENT);
                return;
            };
            match self.runtime.block_on(self.vfs.getattr(&path)) {
                Ok(meta) => {
                    let ino = self.inodes.ino_for(&path);
                    let attr = self.attr_of(ino, &path, &meta);
                    reply.entry(&self.options.entry_ttl, &attr, 0);
                }
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn getattr(&mut self, _req: &fuser::Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
            let Some(path) = self.inodes.path_of(ino) else {
                reply.error(libc::ENOENT);
                return;
            };
            match self.attr(ino, &path) {
                Some(attr) => reply.attr(&self.options.attr_ttl, &attr),
                None => reply.error(libc::ENOENT),
            }
        }

        fn readdir(
            &mut self,
            _req: &fuser::Request<'_>,
            ino: u64,
            _fh: u64,
            offset: i64,
            mut reply: ReplyDirectory,
        ) {
            let Some(path) = self.inodes.path_of(ino) else {
                reply.error(libc::ENOENT);
                return;
            };
            let offset = offset.max(0) as usize;
            let parent_ino = path
                .parent()
                .map(|p| self.inodes.ino_for(&p))
                .unwrap_or(1);
            let fetched = self.runtime.block_on(self.vfs.readdir(&path, offset.saturating_sub(2), 1024));
            let entries = match fetched {
                Ok(entries) => entries,
                Err(e) => {
                    reply.error(errno(&e));
                    return;
                }
            };
            // POSIX requires "." and ".."; the kernel does not invent them.
            let mut all: Vec<(u64, FileType, String)> = Vec::with_capacity(entries.len() + 2);
            all.push((ino, FileType::Directory, ".".to_string()));
            all.push((parent_ino, FileType::Directory, "..".to_string()));
            for entry in &entries {
                let child_ino = self.inodes.ino_for(&entry.path);
                let kind = match entry.kind {
                    EntryKind::File => FileType::RegularFile,
                    EntryKind::Dir => FileType::Directory,
                    EntryKind::Symlink => FileType::Symlink,
                };
                all.push((child_ino, kind, entry.path.file_name().unwrap_or("").to_string()));
            }
            for (i, (child_ino, kind, name)) in all.iter().enumerate().skip(offset) {
                // Offset is the index of the NEXT entry.
                if reply.add(*child_ino, (i + 1) as i64, *kind, name) {
                    break;
                }
            }
            reply.ok();
        }

        fn read(
            &mut self,
            _req: &fuser::Request<'_>,
            ino: u64,
            _fh: u64,
            offset: i64,
            size: u32,
            _flags: i32,
            _lock_owner: Option<u64>,
            reply: ReplyData,
        ) {
            let Some(path) = self.inodes.path_of(ino) else {
                reply.error(libc::ENOENT);
                return;
            };
            match self
                .runtime
                .block_on(self.vfs.read(&path, offset.max(0) as u64, size as usize))
            {
                Ok(bytes) => reply.data(&bytes),
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn write(
            &mut self,
            _req: &fuser::Request<'_>,
            ino: u64,
            _fh: u64,
            offset: i64,
            data: &[u8],
            _write_flags: u32,
            _flags: i32,
            _lock_owner: Option<u64>,
            reply: ReplyWrite,
        ) {
            if self.options.read_only {
                reply.error(libc::EROFS);
                return;
            }
            let Some(path) = self.inodes.path_of(ino) else {
                reply.error(libc::ENOENT);
                return;
            };
            match self.runtime.block_on(self.vfs.write(&path, offset.max(0) as u64, data)) {
                Ok(n) => reply.written(n as u32),
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn create(
            &mut self,
            _req: &fuser::Request<'_>,
            parent: u64,
            name: &OsStr,
            _mode: u32,
            _umask: u32,
            _flags: i32,
            reply: ReplyCreate,
        ) {
            if self.options.read_only {
                reply.error(libc::EROFS);
                return;
            }
            let Some(path) = self.resolve(parent, name) else {
                reply.error(libc::EINVAL);
                return;
            };
            match self.runtime.block_on(self.vfs.create(&path, EntryKind::File)) {
                Ok(handle) => {
                    let ino = self.inodes.ino_for(&path);
                    match self.attr(ino, &path) {
                        Some(attr) => reply.created(&self.options.entry_ttl, &attr, 0, handle.id, 0),
                        None => reply.error(libc::EIO),
                    }
                }
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn mkdir(
            &mut self,
            _req: &fuser::Request<'_>,
            parent: u64,
            name: &OsStr,
            _mode: u32,
            _umask: u32,
            reply: ReplyEntry,
        ) {
            if self.options.read_only {
                reply.error(libc::EROFS);
                return;
            }
            let Some(path) = self.resolve(parent, name) else {
                reply.error(libc::EINVAL);
                return;
            };
            match self.runtime.block_on(self.vfs.create(&path, EntryKind::Dir)) {
                Ok(_) => {
                    let ino = self.inodes.ino_for(&path);
                    match self.attr(ino, &path) {
                        Some(attr) => reply.entry(&self.options.entry_ttl, &attr, 0),
                        None => reply.error(libc::EIO),
                    }
                }
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn unlink(&mut self, _req: &fuser::Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
            self.remove(parent, name, reply);
        }

        fn rmdir(&mut self, _req: &fuser::Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
            self.remove(parent, name, reply);
        }

        fn rename(
            &mut self,
            _req: &fuser::Request<'_>,
            parent: u64,
            name: &OsStr,
            newparent: u64,
            newname: &OsStr,
            _flags: u32,
            reply: ReplyEmpty,
        ) {
            if self.options.read_only {
                reply.error(libc::EROFS);
                return;
            }
            let (Some(from), Some(to)) = (self.resolve(parent, name), self.resolve(newparent, newname))
            else {
                reply.error(libc::EINVAL);
                return;
            };
            match self.runtime.block_on(self.vfs.rename(&from, &to)) {
                Ok(()) => {
                    self.inodes.rename(&from, &to);
                    reply.ok();
                }
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn setattr(
            &mut self,
            _req: &fuser::Request<'_>,
            ino: u64,
            _mode: Option<u32>,
            _uid: Option<u32>,
            _gid: Option<u32>,
            size: Option<u64>,
            _atime: Option<fuser::TimeOrNow>,
            _mtime: Option<fuser::TimeOrNow>,
            _ctime: Option<SystemTime>,
            _fh: Option<u64>,
            _crtime: Option<SystemTime>,
            _chgtime: Option<SystemTime>,
            _bkuptime: Option<SystemTime>,
            _flags: Option<u32>,
            reply: ReplyAttr,
        ) {
            let Some(path) = self.inodes.path_of(ino) else {
                reply.error(libc::ENOENT);
                return;
            };
            if let Some(size) = size {
                if self.options.read_only {
                    reply.error(libc::EROFS);
                    return;
                }
                if let Err(e) = self.runtime.block_on(self.vfs.truncate(&path, size)) {
                    reply.error(errno(&e));
                    return;
                }
            }
            match self.attr(ino, &path) {
                Some(attr) => reply.attr(&self.options.attr_ttl, &attr),
                None => reply.error(libc::ENOENT),
            }
        }

        fn open(&mut self, _req: &fuser::Request<'_>, _ino: u64, flags: i32, reply: ReplyOpen) {
            if self.options.read_only && (flags & libc::O_ACCMODE) != libc::O_RDONLY {
                reply.error(libc::EROFS);
                return;
            }
            reply.opened(0, 0);
        }

        fn flush(&mut self, _req: &fuser::Request<'_>, ino: u64, _fh: u64, _lock: u64, reply: ReplyEmpty) {
            let Some(path) = self.inodes.path_of(ino) else {
                reply.error(libc::ENOENT);
                return;
            };
            match self.runtime.block_on(self.vfs.flush(&path)) {
                Ok(()) => reply.ok(),
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn statfs(&mut self, _req: &fuser::Request<'_>, _ino: u64, reply: ReplyStatfs) {
            match self.runtime.block_on(self.vfs.statfs()) {
                Ok(FsStats { total_bytes, used_bytes, free_bytes }) => {
                    let block = 4096u64;
                    reply.statfs(
                        total_bytes / block,
                        free_bytes / block,
                        free_bytes / block,
                        used_bytes.div_ceil(block).max(1),
                        0,
                        block as u32,
                        255,
                        block as u32,
                    );
                }
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn forget(&mut self, _req: &fuser::Request<'_>, _ino: u64, _nlookup: u64) {
            // Inodes are cheap and the engine is the source of truth; dropping
            // them here would make the next lookup allocate a new number, which
            // confuses open handles. The table is bounded by the paths the
            // kernel has touched.
        }
    }

    impl FuseVfs {
        fn remove(&mut self, parent: u64, name: &OsStr, reply: ReplyEmpty) {
            if self.options.read_only {
                reply.error(libc::EROFS);
                return;
            }
            let Some(path) = self.resolve(parent, name) else {
                reply.error(libc::EINVAL);
                return;
            };
            match self.runtime.block_on(self.vfs.unlink(&path)) {
                Ok(()) => {
                    if let Some(ino) = self.inodes.by_path.get(path.as_str()).copied() {
                        self.inodes.forget(ino);
                    }
                    reply.ok();
                }
                Err(e) => reply.error(errno(&e)),
            }
        }
    }

    /// A running mount. Dropping it unmounts.
    pub struct MountHandle {
        session: Option<fuser::BackgroundSession>,
        mountpoint: PathBuf,
    }

    impl MountHandle {
        /// Where the filesystem is mounted.
        pub fn mountpoint(&self) -> &Path {
            &self.mountpoint
        }

        /// Unmount explicitly. Idempotent.
        pub fn unmount(&mut self) {
            if let Some(session) = self.session.take() {
                session.join();
            }
        }
    }

    impl Drop for MountHandle {
        fn drop(&mut self) {
            self.unmount();
        }
    }

    /// Mount the filesystem and return a handle.
    pub fn mount(vfs: Arc<dyn Vfs>, mountpoint: &Path, options: MountOptions) -> Result<MountHandle> {
        std::fs::create_dir_all(mountpoint).map_err(|e| NidvueError::io(mountpoint, e))?;
        let mut fuse_options = vec![
            fuser::MountOption::FSName(options.fs_name.clone()),
            fuser::MountOption::NoDev,
            fuser::MountOption::NoSuid,
        ];
        if options.read_only {
            fuse_options.push(fuser::MountOption::RO);
        }
        if options.allow_other {
            fuse_options.push(fuser::MountOption::AllowOther);
            // AutoUnmount implies allow_other in fuser; only request it when the
            // user asked for allow_other (and `user_allow_other` is set).
            fuse_options.push(fuser::MountOption::AutoUnmount);
        }
        let fs = FuseVfs::new(vfs, options)?;
        let session = fuser::spawn_mount2(fs, mountpoint, &fuse_options)
            .map_err(|e| NidvueError::Mount(format!("{}: {e}", mountpoint.display())))?;
        Ok(MountHandle { session: Some(session), mountpoint: mountpoint.to_path_buf() })
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use fuse::{mount, FuseVfs, MountHandle};

#[cfg(target_os = "windows")]
mod winfsp {
    use super::*;

    /// A running Windows mount.
    pub struct MountHandle {
        mountpoint: PathBuf,
    }

    impl MountHandle {
        /// Where the filesystem is mounted.
        pub fn mountpoint(&self) -> &Path {
            &self.mountpoint
        }

        /// Unmount explicitly.
        pub fn unmount(&mut self) {
            // The WinFSP host thread stops when the process drops the host.
        }
    }

    /// Mount through WinFSP.
    ///
    /// The adapter implements the same [`Vfs`] contract; the WinFSP host
    /// bindings are a build-time dependency on Windows only. Until that host is
    /// wired, mounting returns a typed error instead of pretending to work.
    pub fn mount(
        _vfs: Arc<dyn Vfs>,
        mountpoint: &Path,
        _options: MountOptions,
    ) -> Result<MountHandle> {
        Err(NidvueError::Unsupported(format!(
            "WinFSP host is not available in this build (mountpoint {})",
            mountpoint.display()
        )))
    }
}

#[cfg(target_os = "windows")]
pub use winfsp::{mount, MountHandle};

/// The FUSE adapter is not available on this platform.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn mount(_vfs: Arc<dyn Vfs>, _mountpoint: &Path, _options: MountOptions) -> Result<MountHandle> {
    Err(NidvueError::Unsupported("no filesystem backend for this platform".into()))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
/// A mount that never started.
pub struct MountHandle;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inodes_start_at_one_and_allocate_upward() {
        let mut table = Inodes::new();
        assert_eq!(table.path_of(1).unwrap().as_str(), "/");
        let ino = table.ino_for(&RemotePath::parse("/a").unwrap());
        assert!(ino >= 2);
        assert_eq!(table.ino_for(&RemotePath::parse("/a").unwrap()), ino);
        assert_eq!(table.path_of(ino).unwrap().as_str(), "/a");
    }

    #[test]
    fn rename_moves_children_inodes() {
        let mut table = Inodes::new();
        let dir = RemotePath::parse("/a").unwrap();
        let child = RemotePath::parse("/a/b").unwrap();
        let dir_ino = table.ino_for(&dir);
        let child_ino = table.ino_for(&child);
        let new_dir = RemotePath::parse("/c").unwrap();
        table.rename(&dir, &new_dir);
        assert_eq!(table.path_of(dir_ino).unwrap().as_str(), "/c");
        assert_eq!(table.path_of(child_ino).unwrap().as_str(), "/c/b");
    }

    #[test]
    fn errno_mapping_is_specific() {
        assert_eq!(errno(&NidvueError::NotFound("x".into())), libc::ENOENT);
        assert_eq!(errno(&NidvueError::AlreadyExists("x".into())), libc::EEXIST);
        assert_eq!(
            errno(&NidvueError::Conflict { path: "x".into(), detail: "directory not empty".into() }),
            libc::ENOTEMPTY
        );
        assert_eq!(errno(&NidvueError::Unsupported("x".into())), libc::EOPNOTSUPP);
        assert_eq!(errno(&NidvueError::Cancelled), libc::EINTR);
        assert_eq!(errno(&NidvueError::Network("x".into())), libc::EIO);
    }
}
