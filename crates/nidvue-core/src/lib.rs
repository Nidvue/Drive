// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Domain types, paths, hashing, configuration and errors shared by every
//! Nidvue Drive crate.
//!
//! This crate depends on no other Nidvue crate: the dependency graph of the
//! workspace is one-directional and `nidvue-core` is its root.
//!
//! ```
//! use nidvue_core::path::RemotePath;
//! let p = RemotePath::parse("/Archive/Photos/a.jpg").unwrap();
//! assert_eq!(p.file_name(), Some("a.jpg"));
//! assert_eq!(p.encoded(), "/Archive/Photos/a.jpg");
//! ```

pub mod config;
pub mod conflict;
pub mod error;
pub mod event;
pub mod hash;
pub mod id;
pub mod metrics;
pub mod path;
pub mod secret;
pub mod time;
pub mod transfers;
pub mod types;

pub use error::{NidvueError, Result};
pub use hash::Hash;
pub use path::RemotePath;
pub use types::{BaselineEntry, ConflictPolicy, EntryKind, EntryMeta, Side, Version};

/// The product name, used in conflict file names and user-facing strings.
pub const PRODUCT: &str = "Nidvue";
/// The crate version reported by `status` and the SDK.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Default Jottacloud device name.
pub const DEFAULT_DEVICE: &str = "Jotta";
/// Default Jottacloud mountpoint.
pub const DEFAULT_MOUNTPOINT: &str = "Archive";
