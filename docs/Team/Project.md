# PROJECT - Nidvue Drive

Owner: CEO. Single source of truth for *what done means*. No task closes without
mapping to an acceptance criterion here.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## Objective

Nidvue Drive is the Nidvue desktop client for Jottacloud: one Rust engine that
does two-way sync and serves a mountable filesystem from a size-limited,
content-addressed cache, exposed through a stable SDK, a WASM plugin host, a
headless CLI and a Qt 6 desktop application. "Finished" for `v0.1.0` means every
criterion below is verified by an executed command at a recorded revision, and
every requirement that is not met is written down with its reason in
[`../deviations.md`](../deviations.md).

## Brief

### Original prompt (verbatim)

> rewrite this: in this folder create a project Nidvue Drive under the master
> brand Nidvue, which is a dual way realtime cross platform desktop client for
> jotta cloud, it fully use high parallization and high performance and highly
> async and high speed with local dynamic cache with configurable limited size,
> it should be separated into comprehensive and systematic sync engine part, sdk
> part, desktop ui part, and also plugin system for extensions, it should fully
> support mouting jotta cloud to a local folder with high speed local cache for
> sync, the app is developed based on rust and qt for ui

### Intention

- Who asks, and for whom: the Nidvue owner, for Jottacloud users who want a
  real desktop client (sync *and* mount) rather than a one-way backup tool.
- Job to be done: keep a local folder and a Jottacloud folder identical in both
  directions, and make the whole account usable as a local path without waiting
  for a full download.
- What "good" looks like when it is finished: a warm read is served from disk, a
  conflict keeps both versions, a crash loses no journal entry, and the UI never
  performs I/O on the UI thread.
- Non-goals: a Jottacloud web UI, server-side search, sharing/permission
  management, mobile clients, an official relationship with Jottacloud AS.
- Requirements the prompt only implies: authentication (a Personal Login Token
  is the only documented flow for command-line clients), rate limiting and
  backoff (the API throttles), an offline story, and an upgrade path for the
  cache index.
- The misreading we must not make: "real-time" does not mean a socket.
  Jottacloud has no webhooks and no change feed, so inbound changes are polled
  and the UI says "checking for remote changes…" instead of pretending.

### Rewritten brief

1. Objective: one engine that syncs and mounts Jottacloud, shipped as a Rust
   workspace with a Qt UI, verified by `cargo xtask ci` on Linux and Windows.
2. Deliverables: `crates/nidvue-{core,jotta,cache,vfs,sync,mount,plugin,sdk,ffi,cli}`,
   `apps/nidvue-ui`, `plugins/example-*`, `benches/`, `tests/invariants`,
   `packaging/`, `xtask/`, `docs/`.
3. Acceptance criteria: AC-BUILD … AC-INVARIANTS, recorded as machine records
   (`teamwork.py criteria list`) and restated under *Definition of done*.
4. Constraints and invariants: AGPL-3.0-only with SPDX headers on every tracked
   file; no credential in a log, a crash report or the cache; `core` depends on
   no Nidvue crate; only `jotta` speaks HTTP; only `mount` touches the kernel;
   the UI performs no I/O.
5. Interfaces to freeze: `nidvue-sdk` (Rust API + events + JSON schema),
   `nidvue-ffi` (the C ABI the UI crosses), the JFS XML codecs, and the
   `Engine` facade the CLI and UI share. Owner: Architect.
6. Lanes: jotta/auth, cache, vfs+mount, sync, plugin, sdk/ffi/cli, ui, docs.
7. Unknowns and assumptions: the API is undocumented and unsupported (isolated
   in one crate); macOS runners are unavailable to this organisation; a real
   account is not available to CI.
8. Evidence plan: every criterion names a command; `Tests.md` maps the layers to
   their owning role; `Archive.md` holds the accepted result per criterion.

### Research that changed the brief

- Jottacloud exposes no webhooks, no change feed and no delta token — the design
  is a budgeted adaptive poll plus a local scan cursor, and the UI states it.
  Evidence: [`../jotta-api.md`](../jotta-api.md) §5.
- The documented command-line authentication flow is the Personal Login Token
  exchanged at the OIDC endpoint — no password is collected or stored. Evidence:
  [`../jotta-api.md`](../jotta-api.md) §1, ADR 0005.
- Allocation needs the MD5 before the body is sent, so hashing is streaming and
  off the async reactor. Evidence: [`../jotta-api.md`](../jotta-api.md) §3.

## Definition of done

| id | criterion (short) | proof |
|---|---|---|
| AC-BUILD | `cargo xtask ci` exits 0 on Linux x86_64 | `cargo xtask ci` |
| AC-LOGIN | token exchange, encrypted persistence, refresh-on-401 | `cargo test -p nidvue-jotta` |
| AC-JFS | JFS XML codecs round-trip, unknown elements ignored, typed errors | `cargo test -p nidvue-jotta` |
| AC-UPLOAD | `files/v1/allocate` resume + MD5/size verification | `cargo test -p nidvue-jotta` |
| AC-RATELIMIT | token bucket, jittered backoff, single-flight, AIMD | `cargo test -p nidvue-jotta` |
| AC-CACHE | content-addressed blobs + index + LRU-K + hard cap | `cargo test -p nidvue-cache` |
| AC-VFS | read-through, spooled writes, atomic downloads | `cargo test -p nidvue-vfs` |
| AC-SYNC | journal-first WAL, three-way plan, crash recovery | `cargo test -p nidvue-sync` |
| AC-CONFLICT | conflict copy, never a silent loss | `cargo test -p nidvue-sync` |
| AC-MOUNT | real FUSE3 mount serves the full op set | `cargo test -p nidvue-mount -- --include-ignored` |
| AC-PLUGIN | wasmtime host, capabilities, fuel/memory limits | `cargo test -p nidvue-plugin` |
| AC-SDK | documented API, examples, C header, JSON schema | `cargo test -p nidvue-sdk`, `cargo doc` |
| AC-CLI | login/mount/sync/status/doctor, `status --json` | `nidvue-cli --json status` |
| AC-UI | Qt6/QML builds, launches, typed state over the C ABI | `cargo xtask ui && cargo xtask ui-smoke` |
| AC-PERF | p50 budgets measured, misses recorded with numbers | `cargo bench -p nidvue-benches` |
| AC-TESTS | unit + proptest + fake-Jotta + crash recovery pass | `cargo test --workspace` |
| AC-DOCS | README, ARCHITECTURE, ADRs, deviations match merged code | `teamwork.py links` |
| AC-PACKAGE | AppImage/deb, dmg, MSI recipes with mount helper permissions | `bash packaging/linux.sh` |
| AC-INVARIANTS | the seven correctness invariants are executable | `cargo test -p nidvue-invariants` |

All nineteen are `verified` in `Criteria.md` with the command and revision that
proved them; the accepted results are in [Archive](Archive.md).

## Scope

- In: sync, mount, cache, auth, plugin host, SDK, C ABI, Qt UI, CLI, packaging,
  CI, docs.
- Out (not doing): a hosted service, mobile clients, sharing/permission
  management, server-side search, an official Jottacloud integration.
- Constraints: AGPL-3.0-only; Rust 1.95+; Qt 6 for the UI; no paid
  infrastructure; no live Jottacloud account in CI.
- Invariants that must not break: no partial or stale content is ever served as
  complete; the journal precedes every mutation; one worker per path; both
  versions survive a conflict; pinned/dirty data survives eviction; credentials
  stay secret; a panic never crosses the C ABI.

## Metrics

| Metric | Target | How measured |
|---|---|---|
| Warm cached read, 1 MiB | ≤ 15 ms p50 | `cargo bench -p nidvue-benches` (measured 88 µs) |
| Cached `readdir` page, 1000 entries | ≤ 2 ms p50 | `cargo bench -p nidvue-benches` (measured 290 µs) |
| Three-way plan cost | flat per path | `cargo bench` (1.44 µs/path at 100k paths) |
| Workspace test suite | 0 failures | `cargo test --workspace` (156 passed) |
| `cargo xtask ci` | green | CI + local, both OSes |

## Assumptions and open questions

| # | Item | Owner | Status | Resolution |
|---|---|---|---|---|
| 1 | The Jottacloud API is undocumented and unsupported | Architect | accepted | isolated in `nidvue-jotta`; a breaking change is one crate |
| 2 | macOS runners are unavailable to this organisation | Release Engineer | accepted | macOS job omitted; recipe and adapter contract exist ([deviations](../deviations.md) 1b) |
| 3 | No live account in CI | Tester | accepted | a fake Jotta server in `crates/nidvue-jotta/tests` |
| 4 | Keyring integration deferred | Architect | open | `SecretStore` seam; see [Risks](Risks.md) R-3 |

## Sources

- [Jottacloud](https://jottacloud.com) — the service this client talks to, accessed 2026-11-15.
- [`../jotta-api.md`](../jotta-api.md) — the API surface this client implements, in-repo.
- [`../../ARCHITECTURE.md`](../../ARCHITECTURE.md) — the crate contract, in-repo.
