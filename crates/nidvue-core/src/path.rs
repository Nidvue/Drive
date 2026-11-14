// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Remote paths and local path helpers.
//!
//! A [`RemotePath`] is always absolute, `/`-separated, has no trailing slash,
//! no empty segment and no `.`/`..` segment. Normalisation happens once, at
//! construction, so every downstream crate can compare paths as strings.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{NidvueError, Result};

/// An absolute, normalised path inside a Jottacloud mountpoint.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RemotePath(String);

impl RemotePath {
    /// The mountpoint root, `/`.
    pub fn root() -> Self {
        Self("/".to_string())
    }

    /// Parse and normalise a path. Accepts `a/b`, `/a/b/`, `//a//b`.
    pub fn parse(input: &str) -> Result<Self> {
        if input.contains('\0') {
            return Err(NidvueError::InvalidPath("path contains NUL".into()));
        }
        let mut segments = Vec::new();
        for seg in input.split('/') {
            match seg {
                "" | "." => continue,
                ".." => {
                    if segments.pop().is_none() {
                        return Err(NidvueError::InvalidPath(format!(
                            "path escapes the mountpoint root: {input}"
                        )));
                    }
                }
                other => segments.push(other),
            }
        }
        Ok(Self(format!("/{}", segments.join("/"))))
    }

    /// The raw normalised form, always starting with `/`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True for the mountpoint root.
    pub fn is_root(&self) -> bool {
        self.0 == "/"
    }

    /// Path segments below the root.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/').filter(|s| !s.is_empty())
    }

    /// Number of segments below the root.
    pub fn depth(&self) -> usize {
        self.segments().count()
    }

    /// The last segment, `None` at the root.
    pub fn file_name(&self) -> Option<&str> {
        self.0.rsplit('/').find(|s| !s.is_empty())
    }

    /// The parent path, `None` at the root.
    pub fn parent(&self) -> Option<RemotePath> {
        if self.is_root() {
            return None;
        }
        let (head, _) = self.0.rsplit_once('/')?;
        Some(Self(if head.is_empty() { "/".into() } else { head.to_string() }))
    }

    /// Join a single child segment.
    pub fn join(&self, name: &str) -> Result<RemotePath> {
        if name.is_empty() || name.contains('/') || name == "." || name == ".." {
            return Err(NidvueError::InvalidPath(format!("not a single segment: {name:?}")));
        }
        Self::parse(&format!("{}/{}", self.0.trim_end_matches('/'), name))
    }

    /// True when `self` is `other` or lives below it.
    pub fn starts_with(&self, other: &RemotePath) -> bool {
        if other.is_root() {
            return true;
        }
        self.0 == other.0
            || (self.0.starts_with(&other.0) && self.0.as_bytes().get(other.0.len()) == Some(&b'/'))
    }

    /// Percent-encode every segment for use in a URL path, keeping `/`.
    pub fn encoded(&self) -> String {
        if self.is_root() {
            return "/".to_string();
        }
        let mut out = String::with_capacity(self.0.len() + 8);
        for seg in self.segments() {
            out.push('/');
            out.push_str(&encode_segment(seg));
        }
        out
    }
}

impl fmt::Display for RemotePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for RemotePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RemotePath({:?})", self.0)
    }
}

/// Percent-encode one path segment using the RFC 3986 unreserved set.
pub fn encode_segment(seg: &str) -> String {
    let mut out = String::with_capacity(seg.len());
    for b in seg.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b':' | b'@'
            | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' => {
                out.push(b as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Convert a remote path to a relative local path below a sync root.
pub fn remote_to_local(root: &std::path::Path, remote: &RemotePath) -> std::path::PathBuf {
    let mut out = root.to_path_buf();
    for seg in remote.segments() {
        out.push(seg);
    }
    out
}

/// Build a remote path from a local path relative to a sync root.
pub fn local_to_remote(root: &std::path::Path, local: &std::path::Path) -> Result<RemotePath> {
    let rel = local
        .strip_prefix(root)
        .map_err(|_| NidvueError::InvalidPath(format!("{} is outside {}", local.display(), root.display())))?;
    let mut s = String::new();
    for comp in rel.components() {
        let seg = comp
            .as_os_str()
            .to_str()
            .ok_or_else(|| NidvueError::InvalidPath("non-UTF-8 path segment".into()))?;
        s.push('/');
        s.push_str(seg);
    }
    RemotePath::parse(&s)
}

/// True when the path looks like a Nidvue conflict copy.
pub fn is_conflict_copy(name: &str) -> bool {
    name.contains("(conflict Nidvue ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises() {
        assert_eq!(RemotePath::parse("a/b/").unwrap().as_str(), "/a/b");
        assert_eq!(RemotePath::parse("//a///b").unwrap().as_str(), "/a/b");
        assert_eq!(RemotePath::parse("/").unwrap().as_str(), "/");
        assert_eq!(RemotePath::parse("").unwrap().as_str(), "/");
        assert_eq!(RemotePath::parse("/a/./b").unwrap().as_str(), "/a/b");
        assert!(RemotePath::parse("/../etc").is_err());
    }

    #[test]
    fn parent_and_name() {
        let p = RemotePath::parse("/Archive/x/y.txt").unwrap();
        assert_eq!(p.file_name(), Some("y.txt"));
        assert_eq!(p.parent().unwrap().as_str(), "/Archive/x");
        assert_eq!(p.parent().unwrap().parent().unwrap().as_str(), "/Archive");
        assert!(RemotePath::parse("/Archive").unwrap().parent().unwrap().is_root());
        assert!(RemotePath::root().parent().is_none());
        assert_eq!(p.depth(), 3);
    }

    #[test]
    fn prefix_is_segment_aware() {
        let base = RemotePath::parse("/Archive/photos").unwrap();
        assert!(RemotePath::parse("/Archive/photos/a").unwrap().starts_with(&base));
        assert!(RemotePath::parse("/Archive/photos").unwrap().starts_with(&base));
        assert!(!RemotePath::parse("/Archive/photos2").unwrap().starts_with(&base));
    }

    #[test]
    fn joins_one_segment_only() {
        let p = RemotePath::parse("/a").unwrap();
        assert_eq!(p.join("b").unwrap().as_str(), "/a/b");
        assert!(p.join("b/c").is_err());
        assert!(p.join("..").is_err());
        assert_eq!(RemotePath::root().join("a").unwrap().as_str(), "/a");
    }

    #[test]
    fn encodes_url_segments() {
        let p = RemotePath::parse("/Archive/a b/ü#1.txt").unwrap();
        assert_eq!(p.encoded(), "/Archive/a%20b/%C3%BC%231.txt");
        assert_eq!(RemotePath::root().encoded(), "/");
    }

    #[test]
    fn local_round_trip() {
        let root = std::path::Path::new("/sync");
        let r = RemotePath::parse("/a/b c").unwrap();
        let l = remote_to_local(root, &r);
        assert_eq!(l, std::path::PathBuf::from("/sync/a/b c"));
        assert_eq!(local_to_remote(root, &l).unwrap(), r);
    }

    #[test]
    fn conflict_names_are_recognised() {
        assert!(is_conflict_copy("a (conflict Nidvue 2026-11-14 101500).txt"));
        assert!(!is_conflict_copy("a.txt"));
    }
}
