// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Sandboxed plugins.
//!
//! WebAssembly under wasmtime is the default and the only enabled kind in this
//! build: a plugin declares its capabilities in `plugin.toml`, the host checks
//! them at every call, and fuel, memory and wall-clock limits contain a
//! runaway. A trap disables the plugin and is reported; it never aborts the
//! engine or the UI.
//!
//! Native dylibs are refused rather than half-supported: loading one into the
//! process would make it a derivative work that must be AGPL-3.0, and this
//! build has no trusted-dylib host. See `docs/licensing.md`.

pub mod host;
pub mod manifest;

pub use host::{LoadedPlugin, PluginHost};
pub use manifest::{Capabilities, Limits, PluginManifest, API_VERSION, HOOKS};
