// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The C header and the exported symbols must agree.
//!
//! A stale header is a link error on someone else's machine, so it is checked
//! here instead of discovered later.

use std::collections::BTreeSet;
use std::path::Path;

fn exported_symbols() -> BTreeSet<String> {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).unwrap();
    let mut out = BTreeSet::new();
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("pub unsafe extern \"C\" fn ") {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if name.starts_with("nidvue_") {
                out.insert(name);
            }
        }
        if let Some(rest) = line.strip_prefix("pub extern \"C\" fn ") {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if name.starts_with("nidvue_") {
                out.insert(name);
            }
        }
    }
    out
}

fn declared_symbols() -> BTreeSet<String> {
    let header =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("include/nidvue.h")).unwrap();
    let mut out = BTreeSet::new();
    for line in header.lines() {
        let line = line.trim();
        for prefix in ["NidvueClient *", "int32_t ", "void "] {
            if let Some(rest) = line.strip_prefix(prefix) {
                if let Some(idx) = rest.find('(') {
                    let name = rest[..idx].trim();
                    if name.starts_with("nidvue_") {
                        out.insert(name.to_string());
                    }
                }
            }
        }
    }
    out
}

#[test]
fn every_exported_symbol_is_declared_and_vice_versa() {
    let exported = exported_symbols();
    let declared = declared_symbols();
    assert!(!exported.is_empty());
    let missing_in_header: Vec<_> = exported.difference(&declared).cloned().collect();
    let missing_in_lib: Vec<_> = declared.difference(&exported).cloned().collect();
    assert!(missing_in_header.is_empty(), "not in include/nidvue.h: {missing_in_header:?}");
    assert!(missing_in_lib.is_empty(), "declared but not exported: {missing_in_lib:?}");
}

#[test]
fn the_header_carries_its_licence() {
    let header =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("include/nidvue.h")).unwrap();
    assert!(header.contains("SPDX-License-Identifier: AGPL-3.0-only"));
}
