# Nidvue Drive — Architecture

> Unofficial Jottacloud desktop client. AGPL-3.0-only. Jottacloud is a trademark
> of Jottacloud AS; this project is not affiliated with or endorsed by them.

This document is the contract the crates are built against. It states the
dependency direction, the data flow, the durable state, the state machines and
the reasons for the big calls. Each "big call" also has an ADR in `docs/adr/`.

## 1. One engine, two front doors

```
                       ┌──────────────────────────────┐
                       │        nidvue-sdk            │  stable Rust API,
                       │  Client · SyncEngine · Mount │  events, JSON schema
                       └───────┬──────────────┬───────┘
                               │              │
                    ┌──────────▼───┐   ┌──────▼────────┐
                    │ nidvue-ffi   │   │ nidvue-cli    │
                    │  C ABI       │   │ headless ctl  │
                    └──────┬───────┘   └──────┬────────┘
                           │                  │
                    ┌──────▼───────┐          │
                    │ apps/ui (Qt) │          │
                    └──────────────┘          │
                                              │
   ┌──────────────┬──────────────┬────────────┴───┬───────────────┐
   │ nidvue-sync  │  nidvue-vfs  │  nidvue-mount  │ nidvue-plugin │
   │ journal      │  read/write  │  FUSE3/WinFSP  │ WASM host     │
   │ scanner      │  pipeline    │  adapters      │ capabilities  │
   │ reconciler   │              │                │               │
   └──────┬───────┴──────┬───────┴────────┬───────┴───────┬───────┘
          │              │                │               │
   ┌──────▼──────────────▼────────────────▼───────────────▼───────┐
   │                        nidvue-cache                          │
   │   blob store (content-addressed) · SQLite metadata index     │
   │   LRU-K eviction · pins · readahead                          │
   └──────────────────────────────┬───────────────────────────────┘
                                  │
   ┌──────────────────────────────▼───────────────────────────────┐
   │                        nidvue-jotta                          │
   │  auth (OIDC + personal login token) · JFS XML · Allocate      │
   │  token bucket · AIMD · single-flight · retry with jitter      │
   └──────────────────────────────┬───────────────────────────────┘
                                  │
   ┌──────────────────────────────▼───────────────────────────────┐
   │                        nidvue-core                           │
   │  types · paths · hashes · config · errors · events · secrets  │
   └──────────────────────────────────────────────────────────────┘
```

Rules, enforced by review and by the dependency list of each `Cargo.toml`:

| Rule | Why |
|---|---|
| `core` depends on no Nidvue crate | everything may depend on it; it must never depend back |
| only `jotta` speaks HTTP | one place for auth, retry, rate limiting, redaction |
| only `mount` touches the kernel | the VFS semantics are platform-independent |
| `sync` never mentions Qt | the engine is usable headless, from the CLI, from a test |
| the UI performs no I/O | every byte arrives through the FFI bridge as typed state |
| `ffi` is the only crate with `extern "C"` | one ABI surface, one place to audit for panics |

## 2. Modes

**Sync mode** binds a local directory to a remote directory and reconciles
continuously. **Mount mode** exposes the whole mountpoint as a filesystem and
fetches on demand. Both are the same `Engine`: the same cache, the same metadata
index, the same conflict model. A path can be in both — a mount that is also a
sync pair — and the per-path serialisation in §6 is what keeps them honest.

## 3. "Real-time" inbound, stated honestly

Jottacloud has no webhooks and no server-side change feed. A socket is not
available, so the client does not pretend to have one:

```
local change ──inotify/FSEvents/ReadDirectoryChangesW──► debounce ──► journal ──► upload
remote change ──adaptive poll (5 s → 60 s)───────────────► liststream delta ──► download
```

- The poll interval starts at `sync.poll_interval_ms`, doubles on empty polls to
  `sync.max_poll_interval_ms`, and resets to the floor after any change. A
  local change also triggers an immediate poll, so the two directions are not
  symmetric in latency but are symmetric in correctness.
- `liststream` is used for every listing: it streams `<file>` and `<folder>`
  elements and reaches first byte far sooner than `mode=list` on large trees.
- The scan cursor (per mountpoint: last listing time and the set of observed
  paths) is persisted, so a restart does not re-download the world.
- The UI shows `checking for remote changes…` while a poll is in flight, and the
  tray shows the current interval. No spinner is ever presented as a socket.

## 4. Durability: journal first

Every mutation is appended to a crash-safe write-ahead journal **before** it is
applied, and marked committed after:

```
intent ──► Journal(append, fsync) ──► apply (temp + fsync + atomic rename) ──► Journal(commit)
```

- The journal is a SQLite database in WAL mode (`nidvue-sync::journal`), one row
  per intent with the correlation id, path, direction, expected hash and state.
- Recovery reads the journal, finds rows in `intent`/`applied` without a
  `commit`, and resolves each deterministically: an upload is re-verified
  against the server's MD5 before it is retried; a download is restarted from
  the temp file's prefix if the server supports resume, otherwise from zero.
  Nothing is assumed to have happened because it was attempted.
- The blob store and the spool use the same discipline: write to a sibling temp
  file, `fsync`, `rename`, `fsync` the directory.

## 5. Three-way reconciliation

For every path the planner has three observations: local state, remote state and
the last-synced baseline. A plan is a pure function of the three:

| local | remote | baseline | action |
|---|---|---|---|
| absent | present | absent | `Create` locally |
| present | absent | absent | `Create` remotely |
| changed | unchanged | present | `Upload` |
| unchanged | changed | present | `Download` |
| changed | changed | present | `Conflict` |
| absent | present | present | `Delete` locally |
| present | absent | present | `Delete` remotely |
| absent | absent | present | `Forget` baseline |
| unchanged | unchanged | present | `Skip` |

"Changed" means `Version::content_differs` — hash first, size second, mtime only
when no hash is available. `mtime` alone is never trusted, because editors and
`rsync -t` preserve it.

The planner is pure and exhaustively tested: `proptest` generates arbitrary
triples of states, applies the plan to a model of both sides, and asserts that
(a) the two sides converge, (b) no file that existed on either side disappears
without a recorded delete, and (c) a re-plan of the result is empty (idempotence).

## 6. Scheduling

- One bounded worker pool (`sync.workers`), work-stealing, priority
  `interactive > mount demand > background`.
- **Per-path serialisation**: a keyed async mutex per path, taken for the whole
  unit of work. Two workers never touch one path; a rename takes both the old
  and the new path in a deterministic order, which is what prevents deadlock.
- Bandwidth is capped by a shared token bucket across all transfers; pausing the
  engine drains in-flight work and stops taking new work.
- Offline is a state, not an error: the queue is durable, work accumulates, and
  the engine resumes without user action.

## 7. Cache

Two stores, both authoritative for different things:

- **Blob store**: content-addressed by SHA-256, sharded two levels
  (`blobs/ab/cd/<hash>`), so no directory grows past a few thousand entries.
  A blob is immutable; a new version is a new blob.
- **Metadata index**: SQLite — path → current blob hash, size, mtime, baseline
  state, access counters. Nothing else is authoritative about *what* a path
  currently is.

Reads are read-through: a warm blob never touches the network. Writes land in
the cache and a durable spool, then drain to the cloud; the spool is bounded, and
only when it is full does a write fail with `EIO` (never a hang, never silent
loss).

Eviction is segmented LRU-K: a probation segment and a protected segment,
promotion on the second access within a window, plus an access-frequency
histogram. Pins are honoured absolutely: a pinned, dirty, or open blob is never
evicted. High/low watermarks keep eviction incremental — it runs in small
batches between I/O, never as a stop-the-world sweep — and the hard cap is
enforced exactly by refusing a write that would exceed it after eviction.

## 8. Mount / VFS

One async trait (`nidvue-vfs::Vfs`) with three thin adapters:

| Platform | Backend | Notes |
|---|---|---|
| Linux | FUSE3 via `fuser` | libfuse3 dynamically linked (LGPL-2.1) |
| Windows | WinFSP | GPLv3 + FLOSS exception; permitted, attributed in `NOTICE` |
| macOS | macFUSE | BSD-3-Clause SDK; File Provider is the documented alternative |

Semantics are identical: `lookup`, `getattr`, `readdir` (paged), `read`, `write`,
`create`, `mkdir`, `unlink`, `rmdir`, `rename`, `truncate`, `statfs`, `xattr`
passthrough where the backend supports it. Hard links are not represented (a
Jottacloud object is a single path); advisory locks are local to the mount and
documented as such in `docs/vfs-semantics.md`.

Kernel callbacks are synchronous, so each adapter owns a runtime handle and
dispatches to it; no callback holds a global lock. Per-inode async locks provide
POSIX-ish consistency without serialising unrelated files. Open handles pin
their blobs for the lifetime of the handle, which is what makes eviction safe
under an open `read`.

## 9. Plugins: WASM by default

`nidvue-plugin` hosts WebAssembly modules under `wasmtime` with:

- a manifest (`plugin.toml`) declaring id, version, API version, licence and the
  **capabilities** requested: filesystem paths, network hosts, UI panels, hooks;
- a capability check at every host call — no ambient authority, a plugin gets
  exactly what it declared and the user approved;
- fuel (CPU), wall-clock and memory limits per call; a trap is caught, reported
  and the plugin is disabled, never fatal to the engine;
- hooks: path filter, content transformer, post-download/pre-upload processor,
  conflict resolver, event subscriber, CLI subcommand.

A native dylib is supported behind an explicit "trusted" flag. The reason WASM is
the default is legal as well as technical: a dylib loaded into the process is
arguably a derivative work and must then be AGPL-3.0, while a WASM module
communicating through a defined API is a separate work that may carry any licence
its manifest declares. The UI says exactly this at install time.

## 10. Concurrency and the network

- A token bucket per host caps request rate; a burst is allowed but never a
  retry storm.
- AIMD concurrency: additive increase while requests succeed, multiplicative
  decrease on 429/503/timeouts, bounded by `network.min_concurrency` and
  `network.max_concurrency`.
- Exponential backoff with full jitter, capped, and `Retry-After` honoured.
- Single-flight coalescing: concurrent requests for the same path share one
  in-flight future, so a directory listing does not become 50 `getattr` calls.
- Timeouts everywhere; a stalled socket is cancelled, not waited on.

## 11. Correctness invariants

These are the tests in `tests/invariants/`; each one names its invariant.

1. No file is truncated, half-written, or replaced by an older version.
2. Every completed mutation is fully applied or fully absent after a crash.
3. Two workers never operate on the same path concurrently.
4. A concurrent local+remote edit always yields a conflict copy, never a loss.
5. Eviction never removes a pinned, dirty, or open file.
6. Credentials never appear in logs, crash reports, or unencrypted on disk.
7. An engine panic cannot abort the UI or the mount.

## 12. Observability

`tracing` with a `CorrelationId` on every operation and transfer, rotating file
logs, redaction at the boundary (`SecretString` cannot be formatted into a log),
an in-app log viewer, an opt-in OTLP exporter and an opt-in Prometheus endpoint.
`nidvue-cli status --json` prints the same `StatsSnapshot` the UI renders.

## 13. Licensing strategy (why the architecture looks like this)

- Everything in this repository is AGPL-3.0-only; every source file carries an
  SPDX header and CI fails without one.
- Qt 6 is **dynamically linked**, LGPLv3-compatible modules only. No GPL-only Qt
  module is used (no Qt Charts, Virtual Keyboard, Data Visualization or Graphs);
  the graphs are drawn in QML `Canvas`, which keeps the module set clean.
- libfuse3 is LGPL-2.1 and dynamically linked. WinFSP is GPLv3 with a FLOSS
  exception and is attributed in `NOTICE`. The macFUSE SDK is BSD-3-Clause.
- The dependency allowlist is in `deny.toml` and `cargo deny check licenses` is
  a CI gate. GPLv3 is accepted because it is one-way compatible into AGPLv3.
- **AGPL §13 (network use).** A desktop client does not trigger it. Any future
  server-side component — a sync relay, a hosted index, a web UI — *would*, and
  would then have to offer its source to its users. That is a deliberate
  non-goal: this project has no server component, and adding one is a licence
  decision, not just an engineering one.
