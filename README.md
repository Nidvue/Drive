# Nidvue Drive

**Two-way, real-time sync and a mountable filesystem for Jottacloud — one Rust
engine, one cache, one conflict model, a Qt desktop app, and a plugin system.**

Nidvue Drive is an unofficial desktop client for [Jottacloud](https://jottacloud.com).
It is not affiliated with, endorsed by, or supported by Jottacloud AS.
Jottacloud is a trademark of its owner. Licensed **AGPL-3.0-only**.

```
┌──────────────────────────────────────────────────────────────┐
│  Nidvue Drive                                                │
│  ● sync   Archive/Projects  ⇅  Jottacloud:/Archive/Projects   │
│  ● mount  /home/you/Jottacloud   (cache 3.4 / 20 GB)          │
│  ↑ 12.4 MB/s   ↓ 8.1 MB/s   queue 7   conflicts 1   healthy   │
└──────────────────────────────────────────────────────────────┘
```

## What it does

- **Sync mode** — pick a local folder and a Jottacloud folder; changes flow both
  ways continuously. Local changes are pushed immediately (filesystem events);
  remote changes are found by a budgeted, adaptive poll, because Jottacloud has
  no server-side notifications. The UI says "checking for remote changes…"
  instead of pretending to hold a socket.
- **Mount mode** — the whole mountpoint appears as a local path, served on
  demand from a size-limited, content-addressed cache. Warm reads never touch
  the network.
- **Conflicts are never silent.** If both sides changed one file, both are kept:
  the newest stays at the original path and the other becomes
  `name (conflict Nidvue 2026-11-14 101500).ext`, listed in the Conflict Center
  with a preview and one-click resolve.
- **Plugins** — sandboxed WebAssembly, capability-scoped, with two working
  examples. Native plugins are allowed behind an explicit trusted flag and must
  be AGPL-3.0 (see `docs/licensing.md`).

## Status

`v0.1.0` — first public pre-release. The engine, cache, sync planner, journal,
FUSE3 mount, WASM plugin host, CLI, SDK and Qt UI are implemented and tested;
the Windows (WinFSP) and macOS (macFUSE) mount adapters are compiled on their
platforms and exercised in CI where a mount is possible. What is not done yet is
listed honestly in [`docs/deviations.md`](docs/deviations.md) — with numbers,
not adjectives.

## Quickstart

```sh
# 1. Build (Rust 1.95+, and Qt 6 for the UI)
cargo build --release

# 2. Log in with a Personal Login Token
#    Jottacloud → Settings → Security → "Generate a personal login token"
cargo run -p nidvue-cli -- login --token-stdin

# 3. Check the connection
cargo run -p nidvue-cli -- doctor

# 4. Mount (headless; no UI needed)
cargo run -p nidvue-cli -- mount ~/Jottacloud --cache-size 20G

# 5. Or sync a folder
cargo run -p nidvue-cli -- sync add docs ~/Documents /Archive/Documents
cargo run -p nidvue-cli -- sync run docs

# 6. The desktop app
cargo xtask ui && ./apps/nidvue-ui/build/nidvue-ui
```

`nidvue-cli status --json` is machine-readable for scripts and dashboards.

## Build and test

```sh
cargo xtask ci      # fmt, clippy -D warnings, tests, docs, deny, SPDX, secrets, DCO, Qt UI
cargo xtask test    # the workspace suite
cargo xtask bench   # performance budgets
cargo xtask ui      # Qt/QML app
```

`cargo xtask ci` is the gate every commit must leave green.

## Documentation

| Document | What is in it |
|---|---|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | crate graph, data flow, state machines, the big calls |
| [`docs/adr/`](docs/adr/) | one decision record per irreversible choice |
| [`docs/jotta-api.md`](docs/jotta-api.md) | the Jottacloud API surface this client uses |
| [`docs/licensing.md`](docs/licensing.md) | every dependency and SDK, its licence, how it is linked |
| [`docs/deviations.md`](docs/deviations.md) | what is not done, and why, with numbers |
| [`docs/plugin-guide.md`](docs/plugin-guide.md) | writing a plugin, and the licence rule |
| [`docs/performance.md`](docs/performance.md) | the budgets and the measured results |
| [`docs/packaging.md`](docs/packaging.md) | AppImage/deb, dmg, MSI and the mount helper |
| [`docs/vfs-semantics.md`](docs/vfs-semantics.md) | exactly what the mount guarantees |

## Repository layout

```
crates/nidvue-core     domain types, paths, hashes, config, errors, secrets
crates/nidvue-jotta    auth, JFS XML, Allocate API, rate limiting
crates/nidvue-cache    blob store, SQLite index, eviction, readahead
crates/nidvue-vfs      virtual filesystem, read/write pipeline
crates/nidvue-sync     journal, scanner, reconciler, scheduler
crates/nidvue-mount    FUSE3 / WinFSP / macFUSE adapters
crates/nidvue-plugin   WASM host, manifest, capabilities
crates/nidvue-sdk      stable Rust API, events, JSON schema
crates/nidvue-ffi      C ABI consumed by the Qt app
crates/nidvue-cli      headless control
apps/nidvue-ui         Qt 6 + QML desktop app
plugins/               two AGPL-3.0 plugins
benches/               the performance budgets
xtask/                 build automation (`cargo xtask`)
```

## Licence, trademark, contributions

- Code: **AGPL-3.0-only** (`LICENSE`). Network use of a modified version
  requires offering the source; this project has no server component.
- The **Nidvue** name and logo are not granted by the licence — forks must be
  renamed (`TRADEMARK.md`).
- Contributions use **DCO sign-off** (`git commit -s`), no CLA.
- Security issues: private disclosure only (`SECURITY.md`). Never paste tokens
  into an issue.
