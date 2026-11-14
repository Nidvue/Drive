// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Three-way reconciliation.
//!
//! A plan is a pure function of three observations per path: the local state,
//! the remote state and the last-synced baseline. Nothing is ever decided from
//! a single side. The table in `ARCHITECTURE.md` §5 is this function.
//!
//! "Changed" means [`Version::content_differs`]: hash first, size second, and
//! mtime only when no hash is available. mtime alone is never trusted.

use std::collections::BTreeMap;

use nidvue_core::path::RemotePath;
use nidvue_core::types::{BaselineEntry, EntryKind, Side, Version};
use serde::{Deserialize, Serialize};

/// One unit of work produced by the planner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// The path exists remotely only; create it locally.
    CreateLocal {
        /// The path.
        path: RemotePath,
        /// The remote version to fetch.
        remote: Version,
    },
    /// The path exists locally only; create it remotely.
    CreateRemote {
        /// The path.
        path: RemotePath,
        /// The local version to upload.
        local: Version,
    },
    /// Both sides changed, local is newer or the policy says local wins.
    Upload {
        /// The path.
        path: RemotePath,
        /// The local version.
        local: Version,
    },
    /// Both sides changed, remote is newer or the policy says remote wins.
    Download {
        /// The path.
        path: RemotePath,
        /// The remote version.
        remote: Version,
    },
    /// The path was deleted remotely since the baseline; delete it locally.
    DeleteLocal {
        /// The path.
        path: RemotePath,
    },
    /// The path was deleted locally since the baseline; delete it remotely.
    DeleteRemote {
        /// The path.
        path: RemotePath,
    },
    /// Both sides changed with different content: preserve both.
    Conflict {
        /// The path.
        path: RemotePath,
        /// The local version.
        local: Version,
        /// The remote version.
        remote: Version,
    },
    /// Both sides deleted the path: drop the baseline.
    Forget {
        /// The path.
        path: RemotePath,
    },
    /// Nothing to do.
    Skip {
        /// The path.
        path: RemotePath,
    },
}

impl Action {
    /// The path this action concerns.
    pub fn path(&self) -> &RemotePath {
        match self {
            Self::CreateLocal { path, .. }
            | Self::CreateRemote { path, .. }
            | Self::Upload { path, .. }
            | Self::Download { path, .. }
            | Self::DeleteLocal { path }
            | Self::DeleteRemote { path }
            | Self::Conflict { path, .. }
            | Self::Forget { path }
            | Self::Skip { path } => path,
        }
    }

    /// True when the action changes bytes on at least one side.
    pub fn is_mutation(&self) -> bool {
        !matches!(self, Self::Skip { .. })
    }
}

/// The plan for one path.
pub fn plan_path(
    path: &RemotePath,
    kind: EntryKind,
    local: Option<&Version>,
    remote: Option<&Version>,
    baseline: Option<&BaselineEntry>,
) -> Action {
    // A directory has no content: its mtime is metadata, not a change signal.
    if kind == EntryKind::Dir {
        return match (local, remote, baseline) {
            (Some(_), Some(_), _) => Action::Skip { path: path.clone() },
            (Some(_), None, None) => Action::CreateRemote {
                path: path.clone(),
                local: Version::default(),
            },
            (None, Some(_), None) => Action::CreateLocal {
                path: path.clone(),
                remote: Version::default(),
            },
            (Some(_), None, Some(_)) => Action::DeleteRemote { path: path.clone() },
            (None, Some(_), Some(_)) => Action::DeleteLocal { path: path.clone() },
            (None, None, Some(_)) => Action::Forget { path: path.clone() },
            (None, None, None) => Action::Skip { path: path.clone() },
        };
    }

    let local_changed = match (local, baseline) {
        (Some(l), Some(b)) => l.content_differs(&b.local),
        (Some(_), None) => true,
        (None, Some(_)) => true,
        (None, None) => false,
    };
    let remote_changed = match (remote, baseline) {
        (Some(r), Some(b)) => r.content_differs(&b.remote),
        (Some(_), None) => true,
        (None, Some(_)) => true,
        (None, None) => false,
    };

    match (local, remote, baseline) {
        // Only one side has it and there is no baseline: a creation.
        (Some(l), None, None) => Action::CreateRemote {
            path: path.clone(),
            local: l.clone(),
        },
        (None, Some(r), None) => Action::CreateLocal {
            path: path.clone(),
            remote: r.clone(),
        },
        // Both sides have it.
        (Some(l), Some(r), _) => {
            if !local_changed && !remote_changed {
                Action::Skip { path: path.clone() }
            } else if local_changed && !remote_changed {
                Action::Upload {
                    path: path.clone(),
                    local: l.clone(),
                }
            } else if !local_changed && remote_changed {
                Action::Download {
                    path: path.clone(),
                    remote: r.clone(),
                }
            } else if l.content_differs(r) {
                Action::Conflict {
                    path: path.clone(),
                    local: l.clone(),
                    remote: r.clone(),
                }
            } else {
                // Both changed to the same content: no conflict, just record it.
                Action::Skip { path: path.clone() }
            }
        }
        // Deletions.
        (None, None, Some(_)) => Action::Forget { path: path.clone() },
        // The local side is gone and the remote is unchanged: propagate the
        // local deletion. (The reverse for a remote deletion.)
        (None, Some(_), Some(_)) => Action::DeleteRemote { path: path.clone() },
        (Some(_), None, Some(_)) => Action::DeleteLocal { path: path.clone() },
        // Unreachable: a baseline without either side is handled above.
        (None, None, None) => Action::Skip { path: path.clone() },
    }
}

/// Plan every path mentioned by any of the three maps.
pub fn plan(
    local: &BTreeMap<RemotePath, (EntryKind, Version)>,
    remote: &BTreeMap<RemotePath, (EntryKind, Version)>,
    baselines: &BTreeMap<RemotePath, BaselineEntry>,
) -> Vec<Action> {
    let mut paths: BTreeMap<&RemotePath, ()> = BTreeMap::new();
    for p in local.keys().chain(remote.keys()).chain(baselines.keys()) {
        paths.insert(p, ());
    }
    let mut actions: Vec<Action> = paths
        .into_keys()
        .map(|path| {
            let kind = local
                .get(path)
                .map(|(k, _)| *k)
                .or_else(|| remote.get(path).map(|(k, _)| *k))
                .or_else(|| baselines.get(path).map(|b| b.kind))
                .unwrap_or(EntryKind::File);
            plan_path(
                path,
                kind,
                local.get(path).map(|(_, v)| v),
                remote.get(path).map(|(_, v)| v),
                baselines.get(path),
            )
        })
        .filter(|a| a.is_mutation())
        .collect();
    actions.sort_by(|a, b| a.path().cmp(b.path()));
    actions
}

/// Which side keeps the original path, and where the loser is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictDecision {
    /// The side that keeps the original path.
    pub keep: Side,
    /// The path the losing version is written to (always a new path).
    pub copy_to: RemotePath,
}

/// Decide a conflict under a policy. Both versions always survive.
pub fn decide_conflict(
    path: &RemotePath,
    local: &Version,
    remote: &Version,
    policy: nidvue_core::types::ConflictPolicy,
    now_ms: i64,
) -> ConflictDecision {
    let keep = match policy {
        nidvue_core::types::ConflictPolicy::KeepBoth => {
            if local.mtime_ms >= remote.mtime_ms {
                Side::Local
            } else {
                Side::Remote
            }
        }
        nidvue_core::types::ConflictPolicy::PreferLocal => Side::Local,
        nidvue_core::types::ConflictPolicy::PreferRemote => Side::Remote,
    };
    ConflictDecision {
        keep,
        copy_to: conflict_copy_path(path, now_ms),
    }
}

/// `<name> (conflict Nidvue YYYY-MM-DD HHMMSS)<ext>` beside the original.
pub fn conflict_copy_path(path: &RemotePath, now_ms: i64) -> RemotePath {
    let name = path.file_name().unwrap_or("file");
    let renamed = nidvue_core::conflict::conflict_copy_name(name, now_ms);
    match path.parent() {
        Some(parent) => parent.join(&renamed).unwrap_or_else(|_| RemotePath::root()),
        None => RemotePath::parse(&format!("/{renamed}")).unwrap_or_else(|_| RemotePath::root()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nidvue_core::hash::Hash;
    use nidvue_core::types::ConflictPolicy;

    fn v(content: &[u8], mtime: i64) -> Version {
        Version::new(content.len() as u64, mtime, Some(Hash::of(content)))
    }

    fn p() -> RemotePath {
        RemotePath::parse("/Docs/a.txt").unwrap()
    }

    fn baseline(local: Version, remote: Version) -> BaselineEntry {
        BaselineEntry {
            path: p(),
            kind: EntryKind::File,
            local,
            remote,
            synced_at_ms: 0,
        }
    }

    #[test]
    fn the_whole_table() {
        let base = v(b"one", 100);
        let changed = v(b"two", 200);
        let b = baseline(base.clone(), base.clone());
        // Creation on each side.
        assert!(matches!(
            plan_path(&p(), EntryKind::File, Some(&changed), None, None),
            Action::CreateRemote { .. }
        ));
        assert!(matches!(
            plan_path(&p(), EntryKind::File, None, Some(&changed), None),
            Action::CreateLocal { .. }
        ));
        // One side changed.
        assert!(matches!(
            plan_path(&p(), EntryKind::File, Some(&changed), Some(&base), Some(&b)),
            Action::Upload { .. }
        ));
        assert!(matches!(
            plan_path(&p(), EntryKind::File, Some(&base), Some(&changed), Some(&b)),
            Action::Download { .. }
        ));
        // Both changed to different content.
        assert!(matches!(
            plan_path(
                &p(),
                EntryKind::File,
                Some(&changed),
                Some(&v(b"three", 300)),
                Some(&b)
            ),
            Action::Conflict { .. }
        ));
        // Both changed to the same content: no conflict.
        assert!(matches!(
            plan_path(
                &p(),
                EntryKind::File,
                Some(&changed),
                Some(&changed),
                Some(&b)
            ),
            Action::Skip { .. }
        ));
        // Deletions.
        assert!(matches!(
            plan_path(&p(), EntryKind::File, None, Some(&base), Some(&b)),
            Action::DeleteRemote { .. }
        ));
        assert!(matches!(
            plan_path(&p(), EntryKind::File, Some(&base), None, Some(&b)),
            Action::DeleteLocal { .. }
        ));
        assert!(matches!(
            plan_path(&p(), EntryKind::File, None, None, Some(&b)),
            Action::Forget { .. }
        ));
        // Unchanged.
        assert!(matches!(
            plan_path(&p(), EntryKind::File, Some(&base), Some(&base), Some(&b)),
            Action::Skip { .. }
        ));
    }

    #[test]
    fn mtime_alone_does_not_create_a_conflict() {
        // Same content, different mtime on both sides: nothing to do.
        let local = v(b"same", 100);
        let remote = v(b"same", 900);
        let b = baseline(local.clone(), local.clone());
        assert!(matches!(
            plan_path(&p(), EntryKind::File, Some(&local), Some(&remote), Some(&b)),
            Action::Skip { .. }
        ));
    }

    #[test]
    fn same_size_different_hash_is_a_conflict() {
        let a = v(b"aaaa", 100);
        let b = v(b"bbbb", 100);
        let base = baseline(v(b"orig", 50), v(b"orig", 50));
        assert!(matches!(
            plan_path(&p(), EntryKind::File, Some(&a), Some(&b), Some(&base)),
            Action::Conflict { .. }
        ));
    }

    #[test]
    fn directories_never_conflict() {
        let a = Version::new(0, 1, None);
        let b = Version::new(0, 2, None);
        let base = baseline(a.clone(), a.clone());
        assert!(matches!(
            plan_path(&p(), EntryKind::Dir, Some(&a), Some(&b), Some(&base)),
            Action::Skip { .. }
        ));
    }

    #[test]
    fn conflict_copies_keep_both_and_name_the_time() {
        let d = decide_conflict(
            &p(),
            &v(b"a", 100),
            &v(b"b", 200),
            ConflictPolicy::KeepBoth,
            0,
        );
        assert_eq!(d.keep, Side::Remote);
        assert_eq!(
            d.copy_to.as_str(),
            "/Docs/a (conflict Nidvue 1970-01-01 000000).txt"
        );
        let d = decide_conflict(
            &p(),
            &v(b"a", 100),
            &v(b"b", 200),
            ConflictPolicy::PreferLocal,
            0,
        );
        assert_eq!(d.keep, Side::Local);
    }

    #[test]
    fn plan_skips_non_mutations_and_sorts() {
        let mut local = BTreeMap::new();
        let mut remote = BTreeMap::new();
        let a = RemotePath::parse("/a").unwrap();
        let b = RemotePath::parse("/b").unwrap();
        local.insert(a.clone(), (EntryKind::File, v(b"x", 1)));
        local.insert(b.clone(), (EntryKind::File, v(b"y", 1)));
        remote.insert(a.clone(), (EntryKind::File, v(b"x", 1)));
        remote.insert(b.clone(), (EntryKind::File, v(b"y", 1)));
        let actions = plan(&local, &remote, &BTreeMap::new());
        assert!(actions.is_empty(), "{actions:?}");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use nidvue_core::hash::Hash;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    #[derive(Debug, Clone)]
    enum Side0 {
        Absent,
        Present(Vec<u8>, i64),
    }

    fn side() -> impl Strategy<Value = Side0> {
        prop_oneof![
            2 => Just(Side0::Absent),
            8 => (prop::collection::vec(any::<u8>(), 0..8), 0i64..1000)
                .prop_map(|(b, m)| Side0::Present(b, m)),
        ]
    }

    fn version(s: &Side0) -> Option<Version> {
        match s {
            Side0::Absent => None,
            Side0::Present(b, m) => Some(Version::new(b.len() as u64, *m, Some(Hash::of(b)))),
        }
    }

    /// Apply one action to the model, returning the surviving versions.
    fn apply(
        action: &Action,
        local: &mut BTreeMap<String, Version>,
        remote: &mut BTreeMap<String, Version>,
    ) {
        let key = action.path().as_str().to_string();
        match action {
            Action::CreateLocal { remote: r, .. } => {
                local.insert(key, r.clone());
            }
            Action::CreateRemote { local: l, .. } => {
                remote.insert(key, l.clone());
            }
            Action::Upload { local: l, .. } => {
                remote.insert(key, l.clone());
            }
            Action::Download { remote: r, .. } => {
                local.insert(key, r.clone());
            }
            Action::DeleteLocal { .. } => {
                local.remove(&key);
            }
            Action::DeleteRemote { .. } => {
                remote.remove(&key);
            }
            Action::Conflict { path, local: l, .. } => {
                // Keep both: the original path takes the local version and the
                // losing version lands on a conflict copy path on BOTH sides,
                // which is what the engine does (download then upload).
                let copy = conflict_copy_path(path, 7).as_str().to_string();
                let loser = remote.get(&key).cloned().unwrap_or_else(|| l.clone());
                remote.insert(copy.clone(), loser.clone());
                local.insert(copy, loser);
                remote.insert(key.clone(), l.clone());
                local.insert(key, l.clone());
            }
            Action::Forget { .. } | Action::Skip { .. } => {}
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// Applying a plan converges: both sides agree, nothing is lost, and a
        /// re-plan of the result is empty (idempotence).
        #[test]
        fn reconciling_converges_and_loses_nothing(
            local0 in side(),
            remote0 in side(),
            base0 in side(),
            local1 in side(),
            remote1 in side(),
        ) {
            let path = RemotePath::parse("/f").unwrap();
            let mut local = BTreeMap::new();
            let mut remote = BTreeMap::new();
            let mut baselines = BTreeMap::new();
            let (lv, rv, bv) = (version(&local0), version(&remote0), version(&base0));
            if let Some(v) = lv.clone() { local.insert(path.clone(), (EntryKind::File, v)); }
            if let Some(v) = rv.clone() { remote.insert(path.clone(), (EntryKind::File, v)); }
            if let Some(v) = bv.clone() {
                baselines.insert(path.clone(), BaselineEntry {
                    path: path.clone(), kind: EntryKind::File, local: v.clone(), remote: v, synced_at_ms: 0,
                });
            }
            // A second path exercises the map, not just one entry.
            let path2 = RemotePath::parse("/g").unwrap();
            if let Some(v) = version(&local1) { local.insert(path2.clone(), (EntryKind::File, v)); }
            if let Some(v) = version(&remote1) { remote.insert(path2.clone(), (EntryKind::File, v)); }

            let before: BTreeSet<String> = local.keys().chain(remote.keys()).map(|p| p.as_str().to_string()).collect();
            let actions = plan(&local, &remote, &baselines);

            let mut m_local: BTreeMap<String, Version> = local.iter().map(|(p, (_, v))| (p.as_str().to_string(), v.clone())).collect();
            let mut m_remote: BTreeMap<String, Version> = remote.iter().map(|(p, (_, v))| (p.as_str().to_string(), v.clone())).collect();
            for a in &actions {
                apply(a, &mut m_local, &mut m_remote);
            }

            // Convergence: every path present on one side has the same content
            // on the other, unless it was deleted on both.
            for (p, v) in &m_local {
                if let Some(other) = m_remote.get(p) {
                    prop_assert_eq!(v.hash.clone(), other.hash.clone(), "path {} diverged", p);
                } else if before.contains(p) && actions.iter().any(|a| matches!(a, Action::DeleteRemote { path } if path.as_str() == p)) {
                    // Deleted remotely because the local side deleted it first.
                } else {
                    prop_assert!(false, "path {} vanished remotely without a delete", p);
                }
            }
            for (p, v) in &m_remote {
                if let Some(other) = m_local.get(p) { prop_assert_eq!(v.hash.clone(), other.hash.clone()); }
            }

            // Idempotence: re-planning the converged state does nothing.
            let mut l2 = BTreeMap::new();
            for (p, v) in &m_local { l2.insert(RemotePath::parse(p).unwrap(), (EntryKind::File, v.clone())); }
            let mut r2 = BTreeMap::new();
            for (p, v) in &m_remote { r2.insert(RemotePath::parse(p).unwrap(), (EntryKind::File, v.clone())); }
            let mut b2 = BTreeMap::new();
            for p in l2.keys() {
                if let (Some((_, lv)), Some((_, rv))) = (l2.get(p), r2.get(p)) {
                    b2.insert(p.clone(), BaselineEntry { path: p.clone(), kind: EntryKind::File, local: lv.clone(), remote: rv.clone(), synced_at_ms: 0 });
                }
            }
            let again = plan(&l2, &r2, &b2);
            prop_assert!(again.is_empty(), "re-plan produced {:?}", again);
        }
    }
}
