// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The trusted-dylib host: a native plugin is loaded only when the
//! configuration trusts native code and its manifest declares AGPL-3.0-only.
//!
//! The fixture is compiled here with `cc`, so the test proves the real
//! `dlopen` path rather than a mock of it. It is skipped where no C compiler
//! exists, and says so.

use std::path::{Path, PathBuf};
use std::process::Command;

use nidvue_plugin::PluginHost;

const FIXTURE: &str = "tests/fixtures/native_plugin.c";

fn build_fixture(dir: &Path) -> Option<PathBuf> {
    // Each test builds into its own directory: the tests run in parallel and a
    // shared path let one delete the library another was loading.
    let out = dir.join("libnative_fixture.so");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-o"])
        .arg(&out)
        .arg(&source)
        .status()
        .ok()?;
    status.success().then_some(out)
}

fn install(dir: &Path, license: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let plugin = dir.join("com.example.native");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.toml"),
        format!(
            r#"id = "com.example.native"
name = "Native filter"
version = "0.1.0"
api_version = 1
license = "{license}"
description = "A native path filter."

[capabilities]
paths = []
hosts = []
hooks = ["path_filter", "event"]
ui_panels = []

[limits]
fuel = 1
memory_mb = 1
wall_ms = 1000
"#
        ),
    )
    .unwrap();
    plugin
}

#[test]
fn a_trusted_native_plugin_loads_and_answers_a_hook() {
    let dir = tempfile::tempdir().unwrap();
    let Some(library) = build_fixture(dir.path()) else {
        eprintln!("skipped: no C compiler to build the native fixture");
        return;
    };
    let plugin = install(&dir.path().join("plugins"), "AGPL-3.0-only");
    std::fs::copy(&library, plugin.join("plugin.so")).unwrap();

    let mut host = PluginHost::new().unwrap().with_native(true);
    let id = host.load(&plugin).expect("a trusted AGPL-3.0 plugin loads");
    assert_eq!(id, "com.example.native");

    let allowed = host
        .plugin_mut(&id)
        .unwrap()
        .call_hook("path_filter", r#"{"path":"/Archive/Documents/a.txt"}"#)
        .unwrap();
    assert_eq!(allowed, r#"{"include":true}"#);

    let skipped = host
        .plugin_mut(&id)
        .unwrap()
        .call_hook("path_filter", r#"{"path":"/Archive/cache/x"}"#)
        .unwrap();
    assert_eq!(skipped, r#"{"include":false}"#);
}

#[test]
fn native_is_refused_unless_trusted_and_agpl() {
    let dir = tempfile::tempdir().unwrap();
    let Some(library) = build_fixture(dir.path()) else {
        eprintln!("skipped: no C compiler to build the native fixture");
        return;
    };

    let untrusted = install(dir.path().join("untrusted").as_path(), "AGPL-3.0-only");
    std::fs::copy(&library, untrusted.join("plugin.so")).unwrap();
    let mut host = PluginHost::new().unwrap();
    let error = host.load(&untrusted).unwrap_err().to_string();
    assert!(
        error.contains("plugin.allow_native"),
        "an untrusted native plugin must be refused by name, got: {error}"
    );

    let non_free = install(dir.path().join("nonfree").as_path(), "MIT");
    std::fs::copy(&library, non_free.join("plugin.so")).unwrap();
    let mut host = PluginHost::new().unwrap().with_native(true);
    let error = host.load(&non_free).unwrap_err().to_string();
    assert!(
        error.contains("AGPL-3.0-only"),
        "a non-AGPL native plugin must be refused for its licence, got: {error}"
    );
}

#[test]
fn a_native_plugin_without_the_abi_symbol_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let plugin = install(dir.path(), "AGPL-3.0-only");
    // A file that is not a library at all: the loader must say so, not panic.
    std::fs::write(plugin.join("plugin.so"), b"not an ELF object").unwrap();
    let mut host = PluginHost::new().unwrap().with_native(true);
    let error = host.load(&plugin).unwrap_err().to_string();
    assert!(
        error.contains("plugin.so"),
        "the error must name the file it could not load, got: {error}"
    );
}
