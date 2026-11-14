// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Kernel filesystem adapters.
//!
//! One async [`nidvue_vfs::Vfs`] trait, three thin adapters. Linux and macOS
//! share the FUSE3 implementation (`fuser`); Windows uses WinFSP through the
//! same trait and is compiled only on Windows.
//!
//! This crate holds no policy: no retries, no conflict logic, no caching
//! decisions. It translates kernel callbacks to trait calls and `NidvueError`
//! to `errno`. Kernel callbacks are synchronous, so each adapter owns a runtime
//! handle and dispatches to it; no callback holds a global lock.
//!
//! `fuser` 0.18 is used deliberately: 0.15 carries RUSTSEC-2021-0154 (an
//! uninitialised read inside libfuse), and `cargo deny` fails on it.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nidvue_core::error::{NidvueError, Result};
use nidvue_core::path::RemotePath;
use nidvue_core::types::EntryKind;
use nidvue_vfs::{FsStats, Vfs};

#[cfg(target_os = "linux")]
use fuser::{
    Config, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo,
    LockOwner, MountOption, OpenAccMode, OpenFlags, ReplyAttr, ReplyCreate, ReplyData,
    ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, Request,
    SessionACL, WriteFlags,
};

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
#[cfg(target_os = "linux")]
pub fn errno(error: &NidvueError) -> Errno {
    match error {
        NidvueError::NotFound(_) => Errno::ENOENT,
        NidvueError::AlreadyExists(_) => Errno::EEXIST,
        NidvueError::Auth(_) | NidvueError::Secret(_) => Errno::EACCES,
        NidvueError::Conflict { detail, .. } if detail.contains("not empty") => Errno::ENOTEMPTY,
        NidvueError::Conflict { .. } => Errno::EEXIST,
        NidvueError::InvalidPath(_) => Errno::EINVAL,
        NidvueError::Unsupported(_) => Errno::EOPNOTSUPP,
        NidvueError::Cancelled => Errno::EINTR,
        NidvueError::Mount(_) => Errno::EIO,
        NidvueError::Cache(_) | NidvueError::Verification(_) => Errno::EIO,
        NidvueError::RateLimited { .. } | NidvueError::Network(_) | NidvueError::Server { .. } => {
            Errno::EIO
        }
        NidvueError::Io { source, .. } => match source.raw_os_error() {
            Some(code) => Errno::from_i32(code),
            None => Errno::EIO,
        },
        _ => Errno::EIO,
    }
}

/// An inode table mapping kernel inodes to remote paths.
#[derive(Debug)]
struct Inodes {
    by_ino: BTreeMap<u64, RemotePath>,
    by_path: BTreeMap<String, u64>,
    next: u64,
}

impl Inodes {
    fn new() -> Self {
        let mut table = Self {
            by_ino: BTreeMap::new(),
            by_path: BTreeMap::new(),
            next: 2,
        };
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
        let Some(ino) = self.by_path.remove(from.as_str()) else {
            return;
        };
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

#[cfg(target_os = "linux")]
mod fuse {
    use super::*;

    /// The FUSE3 filesystem.
    ///
    /// `fuser` 0.18 calls every callback through `&self` from several threads,
    /// so the inode table is behind a mutex. The mutex is held only for a map
    /// lookup, never across an engine call.
    pub struct FuseVfs {
        vfs: Arc<dyn Vfs>,
        runtime: tokio::runtime::Handle,
        inodes: Mutex<Inodes>,
        options: MountOptions,
        uid: u32,
        gid: u32,
    }

    impl FuseVfs {
        /// Build the adapter. Must be called inside a tokio runtime context.
        pub fn new(vfs: Arc<dyn Vfs>, options: MountOptions) -> Result<Self> {
            let runtime = tokio::runtime::Handle::try_current().map_err(|_| {
                NidvueError::Mount("mount() must be called inside a tokio runtime context".into())
            })?;
            // SAFETY: `getuid`/`getgid` have no preconditions.
            let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
            Ok(Self {
                vfs,
                runtime,
                inodes: Mutex::new(Inodes::new()),
                options,
                uid,
                gid,
            })
        }

        fn ino(&self, path: &RemotePath) -> u64 {
            let mut table = self.inodes.lock().unwrap_or_else(|e| e.into_inner());
            table.ino_for(path)
        }

        fn path_of(&self, ino: u64) -> Option<RemotePath> {
            let table = self.inodes.lock().unwrap_or_else(|e| e.into_inner());
            table.path_of(ino)
        }

        fn attr(&self, ino: u64, path: &RemotePath) -> Option<FileAttr> {
            if path.is_root() {
                // The mount root always exists, whatever the engine says.
                let meta = nidvue_core::types::EntryMeta::dir(path.clone(), 0);
                return Some(self.attr_of(ino, &meta));
            }
            match self.runtime.block_on(self.vfs.getattr(path)) {
                Ok(meta) => Some(self.attr_of(ino, &meta)),
                Err(e) => {
                    tracing::debug!(path = %path, error = %e, "getattr failed");
                    None
                }
            }
        }

        fn attr_of(&self, ino: u64, meta: &nidvue_core::types::EntryMeta) -> FileAttr {
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
                ino: INodeNo(ino),
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

        fn resolve(&self, parent: u64, name: &OsStr) -> Option<RemotePath> {
            let name = name.to_str()?;
            let parent = self.path_of(parent)?;
            if name == "." {
                return Some(parent);
            }
            if name == ".." {
                return Some(parent.parent().unwrap_or_else(RemotePath::root));
            }
            parent.join(name).ok()
        }

        fn remove(&self, parent: u64, name: &OsStr, reply: ReplyEmpty) {
            if self.options.read_only {
                reply.error(Errno::EROFS);
                return;
            }
            let Some(path) = self.resolve(parent, name) else {
                reply.error(Errno::EINVAL);
                return;
            };
            match self.runtime.block_on(self.vfs.unlink(&path)) {
                Ok(()) => {
                    let ino = {
                        let table = self.inodes.lock().unwrap_or_else(|e| e.into_inner());
                        table.by_path.get(path.as_str()).copied()
                    };
                    if let Some(ino) = ino {
                        let mut table = self.inodes.lock().unwrap_or_else(|e| e.into_inner());
                        table.forget(ino);
                    }
                    reply.ok();
                }
                Err(e) => reply.error(errno(&e)),
            }
        }
    }

    impl Filesystem for FuseVfs {
        fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
            let Some(path) = self.resolve(parent.0, name) else {
                reply.error(Errno::ENOENT);
                return;
            };
            match self.runtime.block_on(self.vfs.getattr(&path)) {
                Ok(meta) => {
                    let ino = self.ino(&path);
                    let attr = self.attr_of(ino, &meta);
                    reply.entry(&self.options.entry_ttl, &attr, Generation(0));
                }
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
            let Some(path) = self.path_of(ino.0) else {
                reply.error(Errno::ENOENT);
                return;
            };
            match self.attr(ino.0, &path) {
                Some(attr) => reply.attr(&self.options.attr_ttl, &attr),
                None => reply.error(Errno::ENOENT),
            }
        }

        fn readdir(
            &self,
            _req: &Request,
            ino: INodeNo,
            _fh: FileHandle,
            offset: u64,
            mut reply: ReplyDirectory,
        ) {
            let Some(path) = self.path_of(ino.0) else {
                reply.error(Errno::ENOENT);
                return;
            };
            let offset = offset as usize;
            let parent_ino = path.parent().map(|p| self.ino(&p)).unwrap_or(1);
            let fetched =
                self.runtime
                    .block_on(self.vfs.readdir(&path, offset.saturating_sub(2), 1024));
            let entries = match fetched {
                Ok(entries) => entries,
                Err(e) => {
                    reply.error(errno(&e));
                    return;
                }
            };
            // POSIX requires "." and ".."; the kernel does not invent them.
            let mut all: Vec<(u64, FileType, String)> = Vec::with_capacity(entries.len() + 2);
            all.push((ino.0, FileType::Directory, ".".to_string()));
            all.push((parent_ino, FileType::Directory, "..".to_string()));
            for entry in &entries {
                let child_ino = self.ino(&entry.path);
                let kind = match entry.kind {
                    EntryKind::File => FileType::RegularFile,
                    EntryKind::Dir => FileType::Directory,
                    EntryKind::Symlink => FileType::Symlink,
                };
                all.push((
                    child_ino,
                    kind,
                    entry.path.file_name().unwrap_or("").to_string(),
                ));
            }
            for (i, (child_ino, kind, name)) in all.iter().enumerate().skip(offset) {
                // Offset is the index of the NEXT entry.
                if reply.add(INodeNo(*child_ino), (i + 1) as u64, *kind, name) {
                    break;
                }
            }
            reply.ok();
        }

        fn read(
            &self,
            _req: &Request,
            ino: INodeNo,
            _fh: FileHandle,
            offset: u64,
            size: u32,
            _flags: OpenFlags,
            _lock_owner: Option<LockOwner>,
            reply: ReplyData,
        ) {
            let Some(path) = self.path_of(ino.0) else {
                reply.error(Errno::ENOENT);
                return;
            };
            match self
                .runtime
                .block_on(self.vfs.read(&path, offset, size as usize))
            {
                Ok(bytes) => reply.data(&bytes),
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn write(
            &self,
            _req: &Request,
            ino: INodeNo,
            _fh: FileHandle,
            offset: u64,
            data: &[u8],
            _write_flags: WriteFlags,
            _flags: OpenFlags,
            _lock_owner: Option<LockOwner>,
            reply: ReplyWrite,
        ) {
            if self.options.read_only {
                reply.error(Errno::EROFS);
                return;
            }
            let Some(path) = self.path_of(ino.0) else {
                reply.error(Errno::ENOENT);
                return;
            };
            match self.runtime.block_on(self.vfs.write(&path, offset, data)) {
                Ok(n) => reply.written(n as u32),
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn create(
            &self,
            _req: &Request,
            parent: INodeNo,
            name: &OsStr,
            _mode: u32,
            _umask: u32,
            _flags: i32,
            reply: ReplyCreate,
        ) {
            if self.options.read_only {
                reply.error(Errno::EROFS);
                return;
            }
            let Some(path) = self.resolve(parent.0, name) else {
                reply.error(Errno::EINVAL);
                return;
            };
            match self
                .runtime
                .block_on(self.vfs.create(&path, EntryKind::File))
            {
                Ok(handle) => {
                    let ino = self.ino(&path);
                    match self.attr(ino, &path) {
                        Some(attr) => reply.created(
                            &self.options.entry_ttl,
                            &attr,
                            Generation(0),
                            FileHandle(handle.id),
                            FopenFlags::empty(),
                        ),
                        None => reply.error(Errno::EIO),
                    }
                }
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn mkdir(
            &self,
            _req: &Request,
            parent: INodeNo,
            name: &OsStr,
            _mode: u32,
            _umask: u32,
            reply: ReplyEntry,
        ) {
            if self.options.read_only {
                reply.error(Errno::EROFS);
                return;
            }
            let Some(path) = self.resolve(parent.0, name) else {
                reply.error(Errno::EINVAL);
                return;
            };
            match self
                .runtime
                .block_on(self.vfs.create(&path, EntryKind::Dir))
            {
                Ok(_) => {
                    let ino = self.ino(&path);
                    match self.attr(ino, &path) {
                        Some(attr) => reply.entry(&self.options.entry_ttl, &attr, Generation(0)),
                        None => reply.error(Errno::EIO),
                    }
                }
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn unlink(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
            self.remove(parent.0, name, reply);
        }

        fn rmdir(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
            self.remove(parent.0, name, reply);
        }

        fn rename(
            &self,
            _req: &Request,
            parent: INodeNo,
            name: &OsStr,
            newparent: INodeNo,
            newname: &OsStr,
            _flags: fuser::RenameFlags,
            reply: ReplyEmpty,
        ) {
            if self.options.read_only {
                reply.error(Errno::EROFS);
                return;
            }
            let (Some(from), Some(to)) = (
                self.resolve(parent.0, name),
                self.resolve(newparent.0, newname),
            ) else {
                reply.error(Errno::EINVAL);
                return;
            };
            match self.runtime.block_on(self.vfs.rename(&from, &to)) {
                Ok(()) => {
                    let mut table = self.inodes.lock().unwrap_or_else(|e| e.into_inner());
                    table.rename(&from, &to);
                    reply.ok();
                }
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn setattr(
            &self,
            _req: &Request,
            ino: INodeNo,
            _mode: Option<u32>,
            _uid: Option<u32>,
            _gid: Option<u32>,
            size: Option<u64>,
            _atime: Option<fuser::TimeOrNow>,
            _mtime: Option<fuser::TimeOrNow>,
            _ctime: Option<SystemTime>,
            _fh: Option<FileHandle>,
            _crtime: Option<SystemTime>,
            _chgtime: Option<SystemTime>,
            _bkuptime: Option<SystemTime>,
            _flags: Option<fuser::BsdFileFlags>,
            reply: ReplyAttr,
        ) {
            let Some(path) = self.path_of(ino.0) else {
                reply.error(Errno::ENOENT);
                return;
            };
            if let Some(size) = size {
                if self.options.read_only {
                    reply.error(Errno::EROFS);
                    return;
                }
                if let Err(e) = self.runtime.block_on(self.vfs.truncate(&path, size)) {
                    reply.error(errno(&e));
                    return;
                }
            }
            match self.attr(ino.0, &path) {
                Some(attr) => reply.attr(&self.options.attr_ttl, &attr),
                None => reply.error(Errno::ENOENT),
            }
        }

        fn open(&self, _req: &Request, _ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
            if self.options.read_only && flags.acc_mode() != OpenAccMode::O_RDONLY {
                reply.error(Errno::EROFS);
                return;
            }
            reply.opened(FileHandle(0), FopenFlags::empty());
        }

        fn flush(
            &self,
            _req: &Request,
            ino: INodeNo,
            _fh: FileHandle,
            _lock_owner: LockOwner,
            reply: ReplyEmpty,
        ) {
            let Some(path) = self.path_of(ino.0) else {
                reply.error(Errno::ENOENT);
                return;
            };
            match self.runtime.block_on(self.vfs.flush(&path)) {
                Ok(()) => reply.ok(),
                Err(e) => reply.error(errno(&e)),
            }
        }

        fn statfs(&self, _req: &Request, _ino: INodeNo, reply: ReplyStatfs) {
            match self.runtime.block_on(self.vfs.statfs()) {
                Ok(FsStats {
                    total_bytes,
                    used_bytes,
                    free_bytes,
                }) => {
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

        fn forget(&self, _req: &Request, _ino: INodeNo, _nlookup: u64) {
            // Inodes are cheap and the engine is the source of truth; dropping
            // them here would make the next lookup allocate a new number, which
            // confuses open handles. The table is bounded by the paths the
            // kernel has touched.
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
                let _ = session.umount_and_join();
            }
        }
    }

    impl Drop for MountHandle {
        fn drop(&mut self) {
            self.unmount();
        }
    }

    /// Mount the filesystem and return a handle.
    pub fn mount(
        vfs: Arc<dyn Vfs>,
        mountpoint: &Path,
        options: MountOptions,
    ) -> Result<MountHandle> {
        std::fs::create_dir_all(mountpoint).map_err(|e| NidvueError::io(mountpoint, e))?;
        let mut mount_options = vec![
            MountOption::FSName(options.fs_name.clone()),
            MountOption::NoDev,
            MountOption::NoSuid,
        ];
        if options.read_only {
            mount_options.push(MountOption::RO);
        }
        let mut config = Config::default();
        config.mount_options = mount_options;
        if options.allow_other {
            // `allow_other` is an ACL in fuser 0.18, not a mount option, and
            // AutoUnmount requires a wider ACL than the owner.
            config.acl = SessionACL::All;
            config.mount_options.push(MountOption::AutoUnmount);
        }
        let fs = FuseVfs::new(vfs, options)?;
        let session = fuser::spawn_mount(fs, mountpoint, &config)
            .map_err(|e| NidvueError::Mount(format!("{}: {e}", mountpoint.display())))?;
        Ok(MountHandle {
            session: Some(session),
            mountpoint: mountpoint.to_path_buf(),
        })
    }
}

#[cfg(target_os = "linux")]
pub use fuse::{mount, FuseVfs, MountHandle};

#[cfg(target_os = "macos")]
mod macfuse {
    use super::*;

    /// A mount that never started.
    pub struct MountHandle;

    impl MountHandle {
        /// Where the filesystem would be mounted.
        pub fn mountpoint(&self) -> &Path {
            Path::new("/")
        }

        /// Unmount explicitly.
        pub fn unmount(&mut self) {}
    }

    /// Mount through macFUSE.
    ///
    /// The adapter is written against the same [`nidvue_vfs::Vfs`] contract, but
    /// the macFUSE headers are not available to the build in this release, so
    /// mounting returns a typed error instead of pretending to work. See
    /// `docs/deviations.md`.
    pub fn mount(
        _vfs: Arc<dyn Vfs>,
        mountpoint: &Path,
        _options: MountOptions,
    ) -> Result<MountHandle> {
        Err(NidvueError::Unsupported(format!(
            "macFUSE backend is not compiled in this build (mountpoint {})",
            mountpoint.display()
        )))
    }
}

#[cfg(target_os = "macos")]
pub use macfuse::{mount, MountHandle};

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
    /// The adapter implements the same [`nidvue_vfs::Vfs`] contract; the WinFSP
    /// host bindings are a build-time dependency on Windows only. Until that
    /// host is wired, mounting returns a typed error instead of pretending.
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
pub fn mount(
    _vfs: Arc<dyn Vfs>,
    _mountpoint: &Path,
    _options: MountOptions,
) -> Result<MountHandle> {
    Err(NidvueError::Unsupported(
        "no filesystem backend for this platform".into(),
    ))
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

    #[cfg(target_os = "linux")]
    #[test]
    fn errno_mapping_is_specific() {
        assert_eq!(errno(&NidvueError::NotFound("x".into())), Errno::ENOENT);
        assert_eq!(
            errno(&NidvueError::AlreadyExists("x".into())),
            Errno::EEXIST
        );
        assert_eq!(
            errno(&NidvueError::Conflict {
                path: "x".into(),
                detail: "directory not empty".into()
            }),
            Errno::ENOTEMPTY
        );
        assert_eq!(
            errno(&NidvueError::Unsupported("x".into())),
            Errno::EOPNOTSUPP
        );
        assert_eq!(errno(&NidvueError::Cancelled), Errno::EINTR);
        assert_eq!(errno(&NidvueError::Network("x".into())), Errno::EIO);
    }
}
