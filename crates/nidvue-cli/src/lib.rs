// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The crate's library half.
//!
//! Dot-path access to the configuration lives in `nidvue_core::config::dot`,
//! where the engine, the C ABI and the CLI all reach it; this module is the
//! name the CLI's own tests use.

pub use nidvue_core::config::dot as config_dot;
