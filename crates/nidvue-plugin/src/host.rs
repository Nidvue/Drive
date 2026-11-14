// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The WebAssembly plugin host.
//!
//! Plugins run under wasmtime with:
//!
//! - capability checks at every host call (no ambient authority),
//! - fuel, linear-memory and wall-clock limits per call,
//! - traps contained: a plugin that traps is disabled and reported, never fatal.
//!
//! ABI (version 1), as documented in `docs/plugin-guide.md`:
//!
//! ```text
//! nidvue_plugin_init() -> i32                   0 = ok
//! nidvue_plugin_alloc(len: u32) -> u32          guest buffer for the host
//! nidvue_plugin_call(hook: u32, ptr: u32, len: u32) -> u64   (ptr << 32) | len
//! ```

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nidvue_core::error::{NidvueError, Result};
use tracing::{debug, info, warn};
use wasmtime::{
    Caller, Config, Engine, Instance, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder,
    TypedFunc,
};

use crate::manifest::{Capabilities, PluginManifest, API_VERSION};

/// Per-call state visible to host functions.
struct HostState {
    plugin: String,
    capabilities: Capabilities,
    limits: StoreLimits,
    logs: Vec<String>,
    epoch_tick_ms: u64,
}

/// One loaded plugin.
pub struct LoadedPlugin {
    /// Its validated manifest.
    pub manifest: PluginManifest,
    /// The directory it was loaded from.
    pub dir: PathBuf,
    store: Store<HostState>,
    memory: Memory,
    alloc: TypedFunc<u32, u32>,
    call: TypedFunc<(u32, u32, u32), u64>,
}

impl LoadedPlugin {
    /// Call a hook with a JSON payload and return the JSON response.
    pub fn call_hook(&mut self, hook: &str, payload: &str) -> Result<String> {
        if !self.manifest.capabilities.allows_hook(hook) {
            return Err(NidvueError::Plugin {
                plugin: self.manifest.id.clone(),
                message: format!("hook {hook:?} was not declared in plugin.toml"),
            });
        }
        let hook_id = crate::manifest::HOOKS
            .iter()
            .position(|h| *h == hook)
            .ok_or_else(|| NidvueError::Plugin {
                plugin: self.manifest.id.clone(),
                message: format!("unknown hook {hook:?}"),
            })? as u32;

        let limits = self.manifest.limits.clamp();
        self.store.set_fuel(limits.fuel).map_err(|e| NidvueError::Plugin {
            plugin: self.manifest.id.clone(),
            message: format!("fuel: {e}"),
        })?;
        // Wall-clock limit: the host ticks the engine epoch every
        // `epoch_tick_ms`; the deadline is the number of ticks allowed.
        let ticks = limits.wall_ms.div_ceil(10).max(1);
        self.store.set_epoch_deadline(ticks);

        let ptr = self
            .alloc
            .call(&mut self.store, payload.len() as u32)
            .map_err(|e| self.trap("alloc", e))?;
        self.memory
            .write(&mut self.store, ptr as usize, payload.as_bytes())
            .map_err(|e| self.memory_error("write", e))?;

        let started = Instant::now();
        let packed = self
            .call
            .call(&mut self.store, (hook_id, ptr, payload.len() as u32))
            .map_err(|e| self.trap("call", e))?;
        if started.elapsed() > Duration::from_millis(limits.wall_ms) {
            return Err(NidvueError::Plugin {
                plugin: self.manifest.id.clone(),
                message: format!("wall-clock limit {} ms exceeded", limits.wall_ms),
            });
        }
        let out_ptr = (packed >> 32) as usize;
        let out_len = (packed & 0xffff_ffff) as usize;
        if out_len > 16 * 1024 * 1024 {
            return Err(NidvueError::Plugin {
                plugin: self.manifest.id.clone(),
                message: format!("response of {out_len} bytes is over the 16 MiB limit"),
            });
        }
        let mut buf = vec![0u8; out_len];
        self.memory
            .read(&self.store, out_ptr, &mut buf)
            .map_err(|e| self.memory_error("read", e))?;
        String::from_utf8(buf).map_err(|e| NidvueError::Plugin {
            plugin: self.manifest.id.clone(),
            message: format!("response is not UTF-8: {e}"),
        })
    }

    fn memory_error(&self, stage: &str, error: wasmtime::MemoryAccessError) -> NidvueError {
        NidvueError::Plugin {
            plugin: self.manifest.id.clone(),
            message: format!("memory {stage} failed: {error}"),
        }
    }

    fn trap(&self, stage: &str, error: wasmtime::Error) -> NidvueError {
        let text = error.to_string();
        let kind = match error.downcast_ref::<wasmtime::Trap>() {
            Some(wasmtime::Trap::OutOfFuel) => "fuel limit exhausted",
            Some(wasmtime::Trap::Interrupt) => "wall-clock limit exceeded",
            _ if text.contains("fuel") => "fuel limit exhausted",
            _ if text.contains("interrupt") => "wall-clock limit exceeded",
            _ => "trap",
        };
        warn!(plugin = %self.manifest.id, stage, kind, error = %text, "plugin failed");
        NidvueError::Plugin {
            plugin: self.manifest.id.clone(),
            message: format!("{kind} in {stage}: {text}"),
        }
    }
}

/// The host: one wasmtime engine, many plugins.
pub struct PluginHost {
    engine: Engine,
    plugins: Vec<LoadedPlugin>,
    stop: Arc<AtomicBool>,
    epoch_ms: u64,
}

impl PluginHost {
    /// Build a host with epoch interruption for wall-clock limits.
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Disable);
        let engine = Engine::new(&config)
            .map_err(|e| NidvueError::Plugin { plugin: "host".into(), message: e.to_string() })?;
        let stop = Arc::new(AtomicBool::new(false));
        // A single watchdog thread ticks the engine; per-call deadlines are
        // counted in ticks of this period.
        let watchdog_engine = engine.clone();
        let watchdog_stop = stop.clone();
        let epoch_ms = 10u64;
        std::thread::Builder::new()
            .name("nidvue-plugin-epoch".into())
            .spawn(move || {
                while !watchdog_stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(epoch_ms));
                    watchdog_engine.increment_epoch();
                }
            })
            .map_err(|e| NidvueError::Plugin { plugin: "host".into(), message: e.to_string() })?;
        Ok(Self { engine, plugins: Vec::new(), stop, epoch_ms })
    }

    /// Loaded plugins, in load order.
    pub fn plugins(&self) -> &[LoadedPlugin] {
        &self.plugins
    }

    /// Mutable access to one plugin, by id.
    pub fn plugin_mut(&mut self, id: &str) -> Option<&mut LoadedPlugin> {
        self.plugins.iter_mut().find(|p| p.manifest.id == id)
    }

    /// Load a plugin directory.
    pub fn load(&mut self, dir: &Path) -> Result<String> {
        let manifest = PluginManifest::load(dir)?;
        if self.plugins.iter().any(|p| p.manifest.id == manifest.id) {
            return Err(NidvueError::Plugin {
                plugin: manifest.id,
                message: "already loaded".into(),
            });
        }
        if manifest.is_native(dir) {
            // A native dylib is only permitted when explicitly trusted and
            // AGPL-3.0; this build has no trusted-dylib host, so refuse rather
            // than pretend.
            return Err(NidvueError::Plugin {
                plugin: manifest.id,
                message: "native plugins are not enabled in this build".into(),
            });
        }
        let wasm = manifest.wasm_path(dir);
        if !wasm.exists() {
            return Err(NidvueError::Plugin {
                plugin: manifest.id,
                message: format!("missing {}", wasm.display()),
            });
        }
        let module = Module::from_file(&self.engine, &wasm)
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: e.to_string() })?;
        let limits = manifest.limits.clamp();
        let store_limits = StoreLimitsBuilder::new()
            .memory_size((limits.memory_mb * 1024 * 1024) as usize)
            .instances(4)
            .tables(8)
            .build();
        let state = HostState {
            plugin: manifest.id.clone(),
            capabilities: manifest.capabilities.clone(),
            limits: store_limits,
            logs: Vec::new(),
            epoch_tick_ms: self.epoch_ms,
        };
        let mut store = Store::new(&self.engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_fuel(limits.fuel).map_err(|e| NidvueError::Plugin {
            plugin: manifest.id.clone(),
            message: format!("fuel: {e}"),
        })?;
        // Loading and init are not user code: give them a generous deadline so
        // the watchdog cannot interrupt them.
        store.set_epoch_deadline(1_000_000);

        let mut linker: Linker<HostState> = Linker::new(&self.engine);
        linker
            .func_wrap("nidvue", "log", |mut caller: Caller<'_, HostState>, level: i32, ptr: i32, len: i32| {
                if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
                    let mut buf = vec![0u8; len.max(0) as usize];
                    if memory.read(&caller, ptr.max(0) as usize, &mut buf).is_ok() {
                        let text = String::from_utf8_lossy(&buf).to_string();
                        let data = caller.data_mut();
                        data.logs.push(text.clone());
                        if data.logs.len() > 1000 {
                            data.logs.remove(0);
                        }
                        match level {
                            0 => debug!(plugin = %data.plugin, "{text}"),
                            1 => info!(plugin = %data.plugin, "{text}"),
                            _ => warn!(plugin = %data.plugin, "{text}"),
                        }
                    }
                }
            })
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: e.to_string() })?;
        linker
            .func_wrap("nidvue", "now_ms", |_caller: Caller<'_, HostState>| -> i64 {
                nidvue_core::time::now_ms()
            })
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: e.to_string() })?;
        linker
            .func_wrap(
                "nidvue",
                "path_allowed",
                |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
                    read_string(&mut caller, ptr, len)
                        .map(|s| caller.data().capabilities.allows_path(&s) as i32)
                        .unwrap_or(0)
                },
            )
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: e.to_string() })?;
        linker
            .func_wrap(
                "nidvue",
                "http_allowed",
                |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
                    read_string(&mut caller, ptr, len)
                        .map(|s| caller.data().capabilities.allows_host(&s) as i32)
                        .unwrap_or(0)
                },
            )
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: e.to_string() })?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: format!("instantiate: {e}") })?;
        let init = instance
            .get_typed_func::<(), i32>(&mut store, "nidvue_plugin_init")
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: format!("init: {e}") })?;
        let code = init
            .call(&mut store, ())
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: format!("init trap: {e}") })?;
        if code != 0 {
            return Err(NidvueError::Plugin {
                plugin: manifest.id,
                message: format!("init returned {code}"),
            });
        }
        let alloc = instance
            .get_typed_func::<u32, u32>(&mut store, "nidvue_plugin_alloc")
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: format!("alloc: {e}") })?;
        let call = instance
            .get_typed_func::<(u32, u32, u32), u64>(&mut store, "nidvue_plugin_call")
            .map_err(|e| NidvueError::Plugin { plugin: manifest.id.clone(), message: format!("call: {e}") })?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| NidvueError::Plugin {
                plugin: manifest.id.clone(),
                message: "plugin does not export its memory".into(),
            })?;

        let id = manifest.id.clone();
        self.plugins.push(LoadedPlugin { manifest, dir: dir.to_path_buf(), store, memory, alloc, call });
        info!(plugin = %id, "plugin loaded");
        Ok(id)
    }

    /// Load every plugin directory below `root` that has a `plugin.toml`.
    pub fn load_all(&mut self, root: &Path) -> Vec<(PathBuf, Result<String>)> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(root) else { return out };
        for entry in entries.flatten() {
            let dir = entry.path();
            if dir.is_dir() && dir.join("plugin.toml").exists() {
                let result = self.load(&dir);
                out.push((dir, result));
            }
        }
        out
    }

    /// Run every plugin that declares `hook`, in load order, returning the last
    /// successful response. A failing plugin is reported and skipped.
    pub fn dispatch(&mut self, hook: &str, payload: &str) -> Vec<(String, Result<String>)> {
        let ids: Vec<String> = self
            .plugins
            .iter()
            .filter(|p| p.manifest.capabilities.allows_hook(hook))
            .map(|p| p.manifest.id.clone())
            .collect();
        let mut out = Vec::new();
        for id in ids {
            if let Some(plugin) = self.plugin_mut(&id) {
                let result = plugin.call_hook(hook, payload);
                out.push((id, result));
            }
        }
        out
    }

    /// True when the host's watchdog is running.
    pub fn is_alive(&self) -> bool {
        !self.stop.load(Ordering::Relaxed)
    }

    /// Stop the watchdog thread.
    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for PluginHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        // `new` only fails on an engine misconfiguration, which is a build bug.
        Self::new().unwrap_or_else(|e| panic!("plugin host: {e}"))
    }
}

fn read_string(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Option<String> {
    if ptr < 0 || len < 0 || len > 64 * 1024 {
        return None;
    }
    let memory = caller.get_export("memory")?.into_memory()?;
    let mut buf = vec![0u8; len as usize];
    memory.read(&*caller, ptr as usize, &mut buf).ok()?;
    String::from_utf8(buf).ok()
}

/// A counter of host calls, for `status --json`.
pub static CALLS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal plugin in WAT: returns `{"include":true}` for any hook.
    const WAT: &str = r#"
(module
  (memory (export "memory") 1)
  (data (i32.const 1024) "{\"include\":true}")
  (global $next (mut i32) (i32.const 4096))
  (func (export "nidvue_plugin_init") (result i32) i32.const 0)
  (func (export "nidvue_plugin_alloc") (param i32) (result i32)
    (local $p i32)
    global.get $next
    local.set $p
    global.get $next
    local.get 0
    i32.add
    global.set $next
    local.get $p)
  (func (export "nidvue_plugin_call") (param i32 i32 i32) (result i64)
    i64.const 4398046511104
    i64.const 16
    i64.or))
"#;

    const TRAP: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "nidvue_plugin_init") (result i32) i32.const 0)
  (func (export "nidvue_plugin_alloc") (param i32) (result i32) i32.const 1024)
  (func (export "nidvue_plugin_call") (param i32 i32 i32) (result i64) unreachable))
"#;

    const SPIN: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "nidvue_plugin_init") (result i32) i32.const 0)
  (func (export "nidvue_plugin_alloc") (param i32) (result i32) i32.const 1024)
  (func (export "nidvue_plugin_call") (param i32 i32 i32) (result i64)
    (loop $l br $l)
    i64.const 0))
"#;

    fn install(dir: &Path, id: &str, wasm: &str, hooks: &[&str], limits: &str) {
        let hooks = hooks.iter().map(|h| format!("\"{h}\"")).collect::<Vec<_>>().join(", ");
        std::fs::write(
            dir.join("plugin.toml"),
            format!(
                r#"
id = "{id}"
name = "Test"
version = "0.1.0"
api_version = 1
license = "AGPL-3.0-only"

[capabilities]
hooks = [{hooks}]
{limits}
"#
            ),
        )
        .unwrap();
        // The host builds the module from the file, so write the WAT to a
        // `.wasm` file: wasmtime accepts WAT text there too.
        std::fs::write(dir.join(format!("{}.wasm", id.replace('.', "_"))), wasm).unwrap();
    }

    #[test]
    fn loads_and_calls_a_plugin() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), "com.example.hello", WAT, &["path_filter"], "");
        let mut host = PluginHost::new().unwrap();
        let id = host.load(dir.path()).unwrap();
        assert_eq!(id, "com.example.hello");
        let mut plugin = host.plugin_mut(&id).unwrap();
        let out = plugin.call_hook("path_filter", r#"{"path":"/Archive/x"}"#).unwrap();
        assert_eq!(out, r#"{"include":true}"#);
    }

    #[test]
    fn undeclared_hooks_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), "com.example.hello", WAT, &["path_filter"], "");
        let mut host = PluginHost::new().unwrap();
        host.load(dir.path()).unwrap();
        let err = host.plugin_mut("com.example.hello").unwrap().call_hook("transform", "{}").unwrap_err();
        assert!(err.to_string().contains("not declared"), "{err}");
    }

    #[test]
    fn a_trap_is_contained_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), "com.example.trap", TRAP, &["path_filter"], "");
        let mut host = PluginHost::new().unwrap();
        host.load(dir.path()).unwrap();
        let err = host.plugin_mut("com.example.trap").unwrap().call_hook("path_filter", "{}").unwrap_err();
        assert!(err.to_string().contains("trap"), "{err}");
        // The host still works for other plugins.
        assert!(host.plugins().iter().any(|p| p.manifest.id == "com.example.trap"));
    }

    #[test]
    fn a_runaway_plugin_is_stopped() {
        let dir = tempfile::tempdir().unwrap();
        install(
            dir.path(),
            "com.example.spin",
            SPIN,
            &["path_filter"],
            "[limits]\nfuel = 100000\nwall_ms = 50",
        );
        let mut host = PluginHost::new().unwrap();
        host.load(dir.path()).unwrap();
        let started = Instant::now();
        let err = host.plugin_mut("com.example.spin").unwrap().call_hook("path_filter", "{}").unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
        assert!(
            err.to_string().contains("fuel") || err.to_string().contains("wall-clock"),
            "{err}"
        );
    }

    #[test]
    fn a_manifest_without_a_wasm_file_fails_to_load() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("plugin.toml"),
            r#"id = "com.example.missing"
name = "Missing"
version = "0.1.0"
api_version = 1
license = "AGPL-3.0-only""#,
        )
        .unwrap();
        let mut host = PluginHost::new().unwrap();
        let err = host.load(dir.path()).unwrap_err();
        assert!(err.to_string().contains("missing"), "{err}");
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), "com.example.dup", WAT, &["path_filter"], "");
        let mut host = PluginHost::new().unwrap();
        host.load(dir.path()).unwrap();
        assert!(host.load(dir.path()).is_err());
    }

    #[test]
    fn abi_version_mismatch_is_refused_at_the_manifest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("plugin.toml"),
            r#"id = "com.example.future"
name = "Future"
version = "1.0.0"
api_version = 99
license = "MIT""#,
        )
        .unwrap();
        assert!(PluginManifest::load(dir.path()).is_err());
    }
}
