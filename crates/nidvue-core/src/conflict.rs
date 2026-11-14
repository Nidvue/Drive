// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Conflict naming and the conflict record model.

use serde::{Deserialize, Serialize};

use crate::time::conflict_timestamp;

/// Build the conflict copy name for a path.
///
/// `a.txt` becomes `a (conflict Nidvue 2026-11-14 101500).txt`; a dotfile
/// `(none)` and an extensionless name keep their shape.
pub fn conflict_copy_name(name: &str, ms: i64) -> String {
    let stamp = conflict_timestamp(ms);
    let suffix = format!(" (conflict Nidvue {stamp})");
    match name.rfind('.') {
        // A leading dot is a hidden file, not an extension.
        Some(idx) if idx > 0 => {
            let (stem, ext) = name.split_at(idx);
            format!("{stem}{suffix}{ext}")
        }
        _ => format!("{name}{suffix}"),
    }
}

/// What the user decided about a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// Keep the local file, discard the remote version.
    KeepLocal,
    /// Keep the remote file, discard the local version.
    KeepRemote,
    /// Keep both (the default, and the only automatic outcome).
    KeepBoth,
}

/// One detected conflict, as shown in the Conflict Center.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    /// Stable id, derived from the path and detection time.
    pub id: String,
    /// The path both sides edited.
    pub path: String,
    /// The local file that was preserved.
    pub local_path: String,
    /// The conflict copy that was written for the losing side.
    pub conflict_path: String,
    /// Which side kept the original path.
    pub kept: crate::types::Side,
    /// Detection time, ms since the epoch.
    pub detected_ms: i64,
    /// Local size in bytes.
    pub local_size: u64,
    /// Remote size in bytes.
    pub remote_size: u64,
    /// Resolution, once the user picked one.
    pub resolution: Option<Resolution>,
}

impl Conflict {
    /// Build a conflict id that is stable for a path and timestamp.
    pub fn id_for(path: &str, detected_ms: i64) -> String {
        format!("C-{:x}-{:x}", fxhash(path), detected_ms)
    }
}

fn fxhash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_extension() {
        let name = conflict_copy_name("report.tar.gz", 1_763_000_000_000);
        assert!(name.starts_with("report.tar (conflict Nidvue "));
        assert!(name.ends_with(").gz"));
    }

    #[test]
    fn dotfiles_and_extensionless() {
        assert_eq!(
            conflict_copy_name(".bashrc", 0),
            ".bashrc (conflict Nidvue 1970-01-01 000000)"
        );
        assert_eq!(
            conflict_copy_name("Makefile", 0),
            "Makefile (conflict Nidvue 1970-01-01 000000)"
        );
    }

    #[test]
    fn names_do_not_collide_across_seconds() {
        let a = conflict_copy_name("x.txt", 0);
        let b = conflict_copy_name("x.txt", 1000);
        assert_ne!(a, b);
        assert!(crate::path::is_conflict_copy(&a));
    }

    #[test]
    fn ids_are_stable() {
        assert_eq!(Conflict::id_for("/a/b", 7), Conflict::id_for("/a/b", 7));
        assert_ne!(Conflict::id_for("/a/b", 7), Conflict::id_for("/a/b", 8));
    }
}
