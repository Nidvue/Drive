# Plugin guide

A plugin extends Nidvue Drive at defined hook points. It runs in a WebAssembly
sandbox with exactly the capabilities its manifest declares — or, when the user
explicitly trusts native code, as a dylib behind a versioned C ABI.

Related: [configuration](configuration.md) · [licensing](licensing.md) · [deviations](deviations.md)

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

## Native plugins (trusted dylibs)

A native plugin is a shared library loaded into the engine's own process. It
speaks the same hooks and the same JSON payloads, through a C ABI:

```c
unsigned nidvue_plugin_abi_version(void);                     /* 1 */
int      nidvue_plugin_init(const char *config_json);         /* 0 = ok */
int      nidvue_plugin_call(const char *hook, const char *payload, char **out);
void     nidvue_plugin_free(char *out);
```

`nidvue_plugin_call` writes a NUL-terminated JSON response into `*out`, allocated
by the plugin, and the host hands it straight back to `nidvue_plugin_free`. A
non-zero return is reported as a plugin failure; the engine keeps running.

Three things must be true before one loads, and each failure names itself:

| Condition | Why | Error when it is not |
|---|---|---|
| `plugin.allow_native = true` in `config.toml` | a dylib runs with the engine's privileges, so trusting one is a decision the user makes explicitly | `native plugins need plugin.allow_native = true` |
| `license = "AGPL-3.0-only"` in the manifest | the library is linked into an AGPL-3.0 process and is a derivative work | `native plugin declares "MIT": it must be AGPL-3.0-only …` |
| `nidvue_plugin_abi_version()` returns `1` | the host speaks one ABI version and refuses anything else | `native ABI 2 is not the 1 this build speaks` |

```sh
# trust native code, then load a plugin directory
nidvue-cli config set plugin.allow_native true
nidvue-cli plugins call com.example.native path_filter '{"path":"/Archive/x"}'
```

The fuel, memory and wall-clock limits in the manifest are wasmtime's and do not
apply to a dylib: a native plugin is only as contained as the process it runs in,
which is what `allow_native` is asking you to accept. The host still checks the
declared hooks and capabilities on every call, and a plugin whose ABI call fails
is reported rather than fatal.

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
