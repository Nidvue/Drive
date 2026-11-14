// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Domain types shared by the sync engine, the cache, the VFS and the SDK.

use serde::{Deserialize, Serialize};

use crate::hash::Hash;
use crate::path::RemotePath;

/// What a path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link (local only; Jottacloud stores the target as a file).
    Symlink,
}

/// Which side of a sync pair a value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// The local filesystem.
    Local,
    /// Jottacloud.
    Remote,
}

impl Side {
    /// The other side.
    pub fn other(self) -> Side {
        match self {
            Side::Local => Side::Remote,
            Side::Remote => Side::Local,
        }
    }
}

/// The observed state of one path on one side.
///
/// Conflict detection compares `hash` and `size`, never `mtime` alone: two
/// editors that preserve mtime still produce different content.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Version {
    /// Size in bytes (0 for directories).
    pub size: u64,
    /// Modification time, ms since the epoch.
    pub mtime_ms: i64,
    /// Content hash, when known.
    pub hash: Option<Hash>,
}

impl Version {
    /// Build a version from size, mtime and an optional hash.
    pub fn new(size: u64, mtime_ms: i64, hash: Option<Hash>) -> Self {
        Self { size, mtime_ms, hash }
    }

    /// True when the two versions have different content.
    ///
    /// When either hash is unknown the comparison falls back to size, and then
    /// to mtime only if the sizes agree.
    pub fn content_differs(&self, other: &Version) -> bool {
        if self.size != other.size {
            return true;
        }
        match (&self.hash, &other.hash) {
            (Some(a), Some(b)) => a != b,
            _ => self.mtime_ms != other.mtime_ms,
        }
    }
}

/// A directory entry with its version and, for the remote side, a revision id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryMeta {
    /// Path inside the mountpoint.
    pub path: RemotePath,
    /// File, directory or symlink.
    pub kind: EntryKind,
    /// Observed state.
    pub version: Version,
    /// Jottacloud revision identifier, when the server provided one.
    pub remote_rev: Option<String>,
}

impl EntryMeta {
    /// A directory entry.
    pub fn dir(path: RemotePath, mtime_ms: i64) -> Self {
        Self { path, kind: EntryKind::Dir, version: Version::new(0, mtime_ms, None), remote_rev: None }
    }

    /// A file entry.
    pub fn file(path: RemotePath, size: u64, mtime_ms: i64, hash: Option<Hash>) -> Self {
        Self {
            path,
            kind: EntryKind::File,
            version: Version::new(size, mtime_ms, hash),
            remote_rev: None,
        }
    }

    /// True for directories.
    pub fn is_dir(&self) -> bool {
        self.kind == EntryKind::Dir
    }
}

/// The last state both sides agreed on. The third leg of three-way merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineEntry {
    /// Path inside the mountpoint.
    pub path: RemotePath,
    /// File or directory.
    pub kind: EntryKind,
    /// The local version recorded at the last successful sync.
    pub local: Version,
    /// The remote version recorded at the same moment.
    pub remote: Version,
    /// When the baseline was written, ms since the epoch.
    pub synced_at_ms: i64,
}

/// What to do when both sides changed the same path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    /// Keep both: newest at the original path, loser as a conflict copy.
    #[default]
    KeepBoth,
    /// Local wins; the remote version is written as a conflict copy.
    PreferLocal,
    /// Remote wins; the local version is written as a conflict copy.
    PreferRemote,
}

impl ConflictPolicy {
    /// Parse from the configuration strings.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "keep_both" | "keep-both" | "both" => Some(Self::KeepBoth),
            "prefer_local" | "prefer-local" | "local" => Some(Self::PreferLocal),
            "prefer_remote" | "prefer-remote" | "remote" => Some(Self::PreferRemote),
            _ => None,
        }
    }

    /// The configuration string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KeepBoth => "keep_both",
            Self::PreferLocal => "prefer_local",
            Self::PreferRemote => "prefer_remote",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_comparison_uses_hashes() {
        let a = Version::new(10, 100, Some(Hash::of(b"a")));
        let b = Version::new(10, 200, Some(Hash::of(b"b")));
        // Same size, different hash, different mtime: still a content change.
        assert!(a.content_differs(&b));
        // Same size, same hash, different mtime: not a content change.
        let c = Version::new(10, 300, Some(Hash::of(b"a")));
        assert!(!a.content_differs(&c));
        // Unknown hashes fall back to size then mtime.
        assert!(Version::new(10, 1, None).content_differs(&Version::new(11, 1, None)));
        assert!(!Version::new(10, 1, None).content_differs(&Version::new(10, 1, None)));
        assert!(Version::new(10, 1, None).content_differs(&Version::new(10, 2, None)));
    }

    #[test]
    fn conflict_policy_round_trip() {
        for p in [ConflictPolicy::KeepBoth, ConflictPolicy::PreferLocal, ConflictPolicy::PreferRemote] {
            assert_eq!(ConflictPolicy::parse(p.as_str()), Some(p));
        }
        assert_eq!(ConflictPolicy::parse("nonsense"), None);
        assert_eq!(ConflictPolicy::default(), ConflictPolicy::KeepBoth);
        assert_eq!(Side::Local.other(), Side::Remote);
    }
}
