// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The two example plugins load and answer their declared hooks.
//!
//! They are built by `cargo xtask plugins`; the test skips them when the wasm
//! files are absent so a plain `cargo test` on a fresh clone stays green.

use std::path::Path;

use nidvue_plugin::PluginHost;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn hello_filters_caches_and_temp_files() {
    let dir = repo_root().join("plugins/example-hello");
    if !dir.join("com_example_hello.wasm").exists() {
        eprintln!("skipping: build the examples with `cargo xtask plugins`");
        return;
    }
    let mut host = PluginHost::new().unwrap();
    let id = host.load(&dir).unwrap();
    let plugin = host.plugin_mut(&id).unwrap();
    let keep = plugin
        .call_hook(
            "path_filter",
            r#"{"path":"/Archive/Documents/a.txt","kind":"file"}"#,
        )
        .unwrap();
    assert_eq!(keep, r#"{"include":true}"#);
    let skip = plugin
        .call_hook("path_filter", r#"{"path":"/Archive/project/target/x.o"}"#)
        .unwrap();
    assert_eq!(skip, r#"{"include":false}"#);
    let tmp = plugin
        .call_hook("path_filter", r#"{"path":"/Archive/a.tmp"}"#)
        .unwrap();
    assert_eq!(tmp, r#"{"include":false}"#);
}

#[test]
fn filter_scopes_to_documents_and_prefers_local() {
    let dir = repo_root().join("plugins/example-filter");
    if !dir.join("com_example_filter.wasm").exists() {
        eprintln!("skipping: build the examples with `cargo xtask plugins`");
        return;
    }
    let mut host = PluginHost::new().unwrap();
    let id = host.load(&dir).unwrap();
    let plugin = host.plugin_mut(&id).unwrap();
    let inside = plugin
        .call_hook("path_filter", r#"{"path":"/Archive/Documents/report.pdf"}"#)
        .unwrap();
    assert_eq!(inside, r#"{"include":true}"#);
    let outside = plugin
        .call_hook("path_filter", r#"{"path":"/Archive/Photos/a.jpg"}"#)
        .unwrap();
    assert_eq!(outside, r#"{"include":false}"#);
    let conflict = plugin
        .call_hook(
            "conflict_resolve",
            r#"{"path":"/Archive/Documents/x","local_hash":"a","remote_hash":"b"}"#,
        )
        .unwrap();
    assert_eq!(conflict, r#"{"winner":"local"}"#);
}

#[test]
fn an_undeclared_hook_is_refused_even_for_a_real_plugin() {
    let dir = repo_root().join("plugins/example-hello");
    if !dir.join("com_example_hello.wasm").exists() {
        return;
    }
    let mut host = PluginHost::new().unwrap();
    let id = host.load(&dir).unwrap();
    let err = host
        .plugin_mut(&id)
        .unwrap()
        .call_hook("pre_upload", "{}")
        .unwrap_err();
    assert!(err.to_string().contains("not declared"), "{err}");
}
