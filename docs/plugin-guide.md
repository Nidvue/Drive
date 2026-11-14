# Plugin guide

A plugin extends Nidvue Drive at defined hook points. It runs in a WebAssembly
sandbox with exactly the capabilities its manifest declares.

## Licence rule, read this first

| Plugin kind | Licence |
|---|---|
| WASM (default) | any licence the manifest declares — it is a separate work behind a defined ABI |
| native dylib (trusted) | **AGPL-3.0-only** — it is loaded into the process and treated as a derivative work |

The UI shows the declared licence at install time. A native plugin without an
AGPL-3.0 licence is refused.

## Layout

```
my-plugin/
├── plugin.toml        # manifest
├── src/lib.rs         # compiled to wasm32-unknown-unknown
└── README.md
```

```toml
id = "com.example.hello"
name = "Hello"
version = "0.1.0"
api_version = 1
license = "AGPL-3.0-only"
description = "Logs every path the engine touches."

[capabilities]
paths = ["/Archive/Documents"]   # read/write only below these remote paths
hosts = []                       # network hosts, empty = no network
hooks = ["path_filter", "event"]
ui_panels = []

[limits]
fuel = 100_000_000
memory_mb = 32
wall_ms = 2_000
```

## ABI (API version 1)

A plugin exports these functions; the host calls them in this order:

```rust
#[no_mangle] pub extern "C" fn nidvue_plugin_init() -> i32;            // 0 = ok
#[no_mangle] pub extern "C" fn nidvue_plugin_alloc(len: u32) -> u32;    // guest buffer
#[no_mangle] pub extern "C" fn nidvue_plugin_call(
    hook: u32, ptr: u32, len: u32) -> u64;                               // (ptr << 32) | len
```

- `nidvue_plugin_alloc` returns a pointer to a guest buffer the host writes into.
- `hook` is one of the declared hooks; `ptr/len` is a JSON request; the return
  packs a JSON response the same way.
- The host imports `nidvue.log(level, ptr, len)`, `nidvue.read(path_ptr, …)` and
  `nidvue.http(host_ptr, …)`; each checks the manifest's capabilities and traps
  on a violation.

### Hook payloads

| Hook | Request | Response |
|---|---|---|
| `path_filter` | `{"path": "/Archive/x", "kind": "file"}` | `{"include": true}` |
| `transform` | `{"path": "…", "data_b64": "…"}` | `{"data_b64": "…"}` |
| `post_download` | `{"path": "…", "local": "…"}` | `{"ok": true}` |
| `pre_upload` | `{"path": "…", "local": "…"}` | `{"ok": true}` |
| `conflict_resolve` | `{"path": "…", "local_hash": "…", "remote_hash": "…"}` | `{"winner": "local"|"remote"|"both"}` |
| `event` | the event envelope as JSON | ignored |

## Limits

Fuel, memory and wall-clock limits come from the manifest and are clamped by the
host. A trap or a limit breach disables the plugin, emits `PluginFailed`, and
never aborts the engine or the mount.

## Examples in this repository

| Plugin | Shows |
|---|---|
| [`plugins/example-hello`](../plugins/example-hello) | a manifest, a path filter, event logging, capability checks |
| [`plugins/example-filter`](../plugins/example-filter) | a folder-scoped path filter and a conflict policy |

Build an example:

```sh
cargo build --target wasm32-unknown-unknown -p nidvue-plugin-hello --release
cp target/wasm32-unknown-unknown/release/nidvue_plugin_hello.wasm plugins/example-hello/
nidvue-cli plugins install plugins/example-hello
```

## Testing a plugin

`nidvue-cli plugins test <dir>` loads the plugin, runs every declared hook with
a sample payload, and prints the result, the fuel used and any capability
violation. A plugin that traps in a test is not installable.
