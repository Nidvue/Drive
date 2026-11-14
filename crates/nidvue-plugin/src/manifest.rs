// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Plugin manifests and capability declarations.
//!
//! A plugin declares exactly what it needs in `plugin.toml`; the host enforces
//! it at every call. There is no ambient authority: a plugin that did not
//! declare a path cannot read it, whatever it asks the host to do.

use std::path::{Path, PathBuf};

use nidvue_core::error::{NidvueError, Result};
use serde::{Deserialize, Serialize};

/// The plugin ABI version this host implements.
pub const API_VERSION: u32 = 1;

/// Hooks a plugin may implement.
pub const HOOKS: &[&str] = &[
    "path_filter",
    "transform",
    "post_download",
    "pre_upload",
    "conflict_resolve",
    "event",
];

/// What a plugin asked for, shown to the user at install time.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Capabilities {
    /// Remote paths the plugin may read or write.
    pub paths: Vec<String>,
    /// Network hosts the plugin may contact.
    pub hosts: Vec<String>,
    /// Hooks the plugin implements.
    pub hooks: Vec<String>,
    /// UI panel ids the plugin may add.
    pub ui_panels: Vec<String>,
}

impl Capabilities {
    /// True when the plugin declared access to a remote path.
    pub fn allows_path(&self, path: &str) -> bool {
        self.paths.iter().any(|allowed| {
            let allowed = allowed.trim_end_matches('/');
            path == allowed || path.starts_with(&format!("{allowed}/"))
        })
    }

    /// True when the plugin declared a host (exact match or a `*.suffix`).
    pub fn allows_host(&self, host: &str) -> bool {
        self.hosts.iter().any(|allowed| {
            allowed == host
                || allowed
                    .strip_prefix("*.")
                    .map(|suffix| host == suffix || host.ends_with(&format!(".{suffix}")))
                    .unwrap_or(false)
        })
    }

    /// True when the plugin declared a hook.
    pub fn allows_hook(&self, hook: &str) -> bool {
        self.hooks.iter().any(|h| h == hook)
    }
}

/// Resource limits, clamped by the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Limits {
    /// WASM fuel per call.
    pub fuel: u64,
    /// Linear memory ceiling in MiB.
    pub memory_mb: u64,
    /// Wall-clock ceiling per call in milliseconds.
    pub wall_ms: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            fuel: 100_000_000,
            memory_mb: 64,
            wall_ms: 2_000,
        }
    }
}

impl Limits {
    /// The host's own ceiling: a manifest cannot ask for more.
    pub fn clamp(&self) -> Limits {
        Limits {
            fuel: self.fuel.clamp(1_000, 10_000_000_000),
            memory_mb: self.memory_mb.clamp(1, 512),
            wall_ms: self.wall_ms.clamp(10, 30_000),
        }
    }
}

/// `plugin.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Reverse-DNS id, e.g. `com.example.hello`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// ABI version.
    pub api_version: u32,
    /// SPDX licence expression.
    pub license: String,
    /// One line for the plugin list.
    #[serde(default)]
    pub description: String,
    /// Declared capabilities.
    #[serde(default)]
    pub capabilities: Capabilities,
    /// Resource limits.
    #[serde(default)]
    pub limits: Limits,
}

impl PluginManifest {
    /// Read and validate `plugin.toml` from a plugin directory.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("plugin.toml");
        let text = std::fs::read_to_string(&path).map_err(|e| NidvueError::io(&path, e))?;
        let manifest: Self = toml::from_str(&text).map_err(|e| NidvueError::Plugin {
            plugin: dir.display().to_string(),
            message: format!("plugin.toml: {e}"),
        })?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest against the host's rules.
    pub fn validate(&self) -> Result<()> {
        let fail = |msg: String| {
            Err(NidvueError::Plugin {
                plugin: self.id.clone(),
                message: msg,
            })
        };
        if self.id.is_empty()
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        {
            return fail("id must be reverse-DNS-ish: alphanumerics, '.', '-', '_'".into());
        }
        if self.api_version != API_VERSION {
            return fail(format!(
                "api_version {} is not supported (host speaks {API_VERSION})",
                self.api_version
            ));
        }
        if self.license.trim().is_empty() {
            return fail("license is required; a plugin must declare one".into());
        }
        if self.name.trim().is_empty() || self.version.trim().is_empty() {
            return fail("name and version are required".into());
        }
        for hook in &self.capabilities.hooks {
            if !HOOKS.contains(&hook.as_str()) {
                return fail(format!("unknown hook {hook:?}"));
            }
        }
        for path in &self.capabilities.paths {
            if !path.starts_with('/') {
                return fail(format!("capability path must be absolute: {path}"));
            }
        }
        for host in &self.capabilities.hosts {
            if host.contains('/') || host.contains(' ') {
                return fail(format!("capability host is not a host: {host}"));
            }
        }
        Ok(())
    }

    /// The wasm file beside the manifest.
    pub fn wasm_path(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{}.wasm", self.id.replace('.', "_")))
    }

    /// True when the manifest allows loading the module beside it.
    pub fn is_native(&self, dir: &Path) -> bool {
        dir.join("plugin.so").exists() || dir.join("plugin.dll").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PluginManifest {
        PluginManifest {
            id: "com.example.hello".into(),
            name: "Hello".into(),
            version: "0.1.0".into(),
            api_version: API_VERSION,
            license: "AGPL-3.0-only".into(),
            description: String::new(),
            capabilities: Capabilities {
                paths: vec!["/Archive/Documents".into()],
                hosts: vec!["api.example.com".into(), "*.jottacloud.com".into()],
                hooks: vec!["path_filter".into()],
                ui_panels: vec![],
            },
            limits: Limits::default(),
        }
    }

    #[test]
    fn valid_manifest_passes() {
        manifest().validate().unwrap();
    }

    #[test]
    fn rejects_bad_fields() {
        let mut m = manifest();
        m.api_version = 99;
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.license = "  ".into();
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.capabilities.hooks = vec!["exec".into()];
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.capabilities.paths = vec!["relative".into()];
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.id = "has space".into();
        assert!(m.validate().is_err());
    }

    #[test]
    fn capability_matching_is_segment_aware() {
        let caps = manifest().capabilities;
        assert!(caps.allows_path("/Archive/Documents"));
        assert!(caps.allows_path("/Archive/Documents/a/b"));
        assert!(!caps.allows_path("/Archive/Documents2"));
        assert!(!caps.allows_path("/Archive/Other"));
        assert!(caps.allows_host("api.example.com"));
        assert!(caps.allows_host("eu.jottacloud.com"));
        assert!(!caps.allows_host("evil.example.com"));
        assert!(caps.allows_hook("path_filter"));
        assert!(!caps.allows_hook("transform"));
    }

    #[test]
    fn limits_are_clamped() {
        let greedy = Limits {
            fuel: u64::MAX,
            memory_mb: 100_000,
            wall_ms: 999_999,
        };
        let clamped = greedy.clamp();
        assert_eq!(clamped.memory_mb, 512);
        assert_eq!(clamped.wall_ms, 30_000);
        assert!(clamped.fuel <= 10_000_000_000);
        let tiny = Limits {
            fuel: 0,
            memory_mb: 0,
            wall_ms: 0,
        }
        .clamp();
        assert!(tiny.fuel >= 1_000 && tiny.memory_mb >= 1 && tiny.wall_ms >= 10);
    }

    #[test]
    fn loads_from_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("plugin.toml"),
            r#"
id = "com.example.hello"
name = "Hello"
version = "0.1.0"
api_version = 1
license = "AGPL-3.0-only"

[capabilities]
hooks = ["path_filter"]
"#,
        )
        .unwrap();
        let m = PluginManifest::load(dir.path()).unwrap();
        assert_eq!(m.id, "com.example.hello");
        assert_eq!(m.limits, Limits::default());
        assert!(m.wasm_path(dir.path()).ends_with("com_example_hello.wasm"));
    }
}
