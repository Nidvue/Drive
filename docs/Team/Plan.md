# PLAN - milestones and task graph

Owner: Project Manager. One row per task; `deps` must be `done` before it runs.
Priority: P0 code, P1 tests, P2 docs.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## State

The delivered work predates the task records: the queue was never populated, so
this plan is written from the evidence that exists — the criteria ledger
(`Criteria.md`, 19 verified), the commit history and the accepted results in
[Archive](Archive.md). From here on every new slice is filed as a task before it
starts (`teamwork.py task add`) so the board is the queue's state, not an opinion.

## M1 - Engine skeleton and the API boundary (done, `e73c89d`)

Exit: `cargo test -p nidvue-jotta -p nidvue-core` passes; no crate depends upward.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0001 | Freeze the crate contract and dependency direction | architect | P0 | - | `ARCHITECTURE.md`, ADR 0001-0005 | done |
| T-0002 | `nidvue-core`: types, paths, hashing, config, errors, secrets | developer | P0 | architect:T-0001 | `crates/nidvue-core` | done |
| T-0003 | `nidvue-jotta`: auth, JFS XML, allocate, rate limiting | developer | P0 | developer:T-0002 | `crates/nidvue-jotta` | done |
| T-0004 | Fake Jotta server + integration tests | tester | P1 | developer:T-0003 | `crates/nidvue-jotta/tests/fake_jotta.rs` | done |

## M2 - Cache, VFS and mount (done, `e73c89d`)

Exit: a warm read never touches the network; a real FUSE mount serves the op set.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0005 | Content-addressed cache, index, LRU-K, hard cap | developer | P0 | developer:T-0002 | `crates/nidvue-cache` | done |
| T-0006 | Read-through VFS, spooled writes, atomic downloads | developer | P0 | developer:T-0005 | `crates/nidvue-vfs` | done |
| T-0007 | FUSE3 adapter on fuser 0.18 (0.15 carries RUSTSEC-2021-0154) | developer | P0 | developer:T-0006 | `crates/nidvue-mount` | done |
| T-0008 | Real-mount test (`--include-ignored`) | tester | P1 | developer:T-0007 | `crates/nidvue-mount/tests/fuse_mount.rs` | done |

## M3 - Sync engine and conflicts (done, `e73c89d`)

Exit: SIGKILL during a mutation loses nothing; a concurrent edit keeps both files.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0009 | Journal-first WAL and three-way reconciler | developer | P0 | developer:T-0005 | `crates/nidvue-sync` | done |
| T-0010 | Conflict copies + Conflict Center record | developer | P0 | developer:T-0009 | `crates/nidvue-sync` | done |
| T-0011 | Crash-recovery and convergence tests (proptest) | tester | P1 | developer:T-0009 | `crates/nidvue-sync/tests/engine.rs` | done |

## M4 - Front doors: plugin, SDK, FFI, CLI, UI (done, `e73c89d`)

Exit: the UI launches with no QML errors and receives typed state over the C ABI.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0012 | WASM plugin host (capabilities, fuel, memory, trap containment) | developer | P0 | developer:T-0002 | `crates/nidvue-plugin` | done |
| T-0013 | Stable SDK: typed events, JSON schema, C header | developer | P0 | developer:T-0009 | `crates/nidvue-sdk` | done |
| T-0014 | C ABI bridge; panic containment at every entry point | developer | P0 | developer:T-0013 | `crates/nidvue-ffi` | done |
| T-0015 | Headless CLI: login, mount, sync, status, doctor | developer | P0 | developer:T-0013 | `crates/nidvue-cli` | done |
| T-0016 | Qt 6 + QML app, tray icon, popup, settings window | developer | P0 | developer:T-0014 | `apps/nidvue-ui` | done |
| T-0017 | UI smoke test (offscreen launch, QML error scan) | tester | P1 | developer:T-0016 | `cargo xtask ui-smoke` | done |

## M5 - Proof, packaging, governance (done, `35ba7a1`)

Exit: `cargo xtask ci` green; every criterion verified; main protected.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0018 | Seven correctness invariants as executable tests | tester | P1 | developer:T-0009 | `tests/invariants` | done |
| T-0019 | Measured budgets and honest deviations | tester | P1 | developer:T-0006 | `docs/performance.md`, `docs/deviations.md` | done |
| T-0020 | Packaging recipes (deb/tar.gz, dmg, MSI) | release-engineer | P1 | developer:T-0015 | `packaging/` | done |
| T-0021 | One green gate: `cargo xtask ci` | release-engineer | P0 | tester:T-0018 | `xtask/` | done |
| T-0022 | CI matrix, CodeQL, cargo-deny, SPDX, secret scan, DCO | release-engineer | P0 | release-engineer:T-0021 | `.github/workflows/` | done |
| T-0023 | Protected `main`, rebase-only PRs, required checks | ceo | P0 | release-engineer:T-0022 | [Decisions](Decisions.md) D-20261115-CEO | done |

## M7 - v0.2.0: close the deviations that can be closed (done, `798a123`)

Exit: `cargo xtask ci` green with `ffi-smoke` in the gate; the five new criteria
verified; `teamwork.py loop` reports STOP and `audit` reports zero problems.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0001 | OS keyring `SecretStore` with a recorded fallback | developer | P0 | - | `crates/nidvue-core/src/secret.rs`, `secret.backend` | done |
| T-0002 | Prometheus `/metrics` and `/healthz` on `log.prometheus_bind` | developer | P0 | - | `crates/nidvue-telemetry` | done |
| T-0003 | OTLP/HTTP JSON exporter on `log.otel_endpoint` | developer | P0 | T-0002 | `crates/nidvue-telemetry` | done |
| T-0004 | `config get|set|list|path` with full key coverage | developer | P1 | - | `crates/nidvue-cli/src/lib.rs` | done |
| T-0005 | Transfer registry, `Status.transfers`, C ABI, `Transfers.qml` | developer | P0 | - | `crates/nidvue-core/src/transfers.rs`, `apps/nidvue-ui/qml/Transfers.qml` | done |
| T-0006 | Cold-read and sustained-transfer budgets measured | tester | P1 | T-0005 | `benches/benches/transfer.rs` | done |
| T-0007 | 100k-path initial sync measured | tester | P1 | - | `benches/benches/initial_sync.rs` | done |
| T-0008 | C consumer and the CLI-to-engine config seam | tester | P1 | T-0004 | `cargo xtask ffi-smoke`, `crates/nidvue-cli/tests/config.rs` | done |
| T-0009 | v0.2.0 version, changelog, release drill | release-engineer | P2 | - | `Cargo.toml`, `CHANGELOG.md` | done |
| T-0010 | Deviations, ARCHITECTURE, README, configuration | product-manager | P2 | - | `docs/` | done |

Not landed, and why: the mount backends (deviation 1, 1b, 2) need a platform
this build environment does not have, the UI frame budget needs a windowed
session, and the live-account and native-dylib items are deliberate refusals.

## M8 - Usable end to end: settings, pair filters, conflicts (done, `33fa2e4`)

Exit: `cargo xtask ci` green on the merged branch; the Conflict Center resolves a
conflict from the UI; every configuration value is reachable from the CLI; a pair
is filtered without editing `config.toml`.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0025 | Dot-path config for the CLI, the SDK and the C ABI, with pair-name segments | developer | P0 | M7 T-0004 | `crates/nidvue-core/src/config.rs` (`config::dot`) | done |
| T-0026 | `sync add --include/--exclude/--conflict-policy/--paused`, `remove`, `pause`, `resume`, `list --json` | developer | P0 | T-0025 | `crates/nidvue-cli/src/main.rs` | done |
| T-0027 | Per-pair exclude globs in Settings, log filter, tray notifications | developer | P1 | T-0026 | `apps/nidvue-ui/qml/SettingsPane.qml`, `Logs.qml`, `tray.cpp` | done |
| T-0028 | Conflict Center resolves: `keep_local`/`keep_remote` parse | developer | P0 | - | `crates/nidvue-core/src/types.rs` | done |
| T-0029 | One configuration lock in the SDK; no stale `config()` | developer | P0 | T-0025 | `crates/nidvue-sdk/src/lib.rs` | done |
| T-0030 | CLI integration tests for `config` and `sync` | tester | P1 | T-0026 | `crates/nidvue-cli/tests/cli.rs` | done |
| T-0031 | Broken-pipe exit instead of a panic | developer | P2 | - | `crates/nidvue-cli/src/main.rs` | done |

## M10 - v0.2.1: every setting from the window, a live cache cap (done, `7d1c7ad`)

Exit: the Advanced card lists every config leaf and the smoke test fails when it
receives none; lowering the cache cap evicts before the call returns; the app
honours `ui.start_minimised` and `ui.theme` before the first frame.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0036 | Advanced settings card over the flat config map | developer | P0 | M8 T-0025 | `apps/nidvue-ui/qml/SettingsPane.qml`, `nidvue_client_config_list_json` | done |
| T-0037 | Live cache cap (`AtomicU64` + `set_capacity`) applied by the SDK | developer | P0 | - | `crates/nidvue-cache/src/lib.rs`, `crates/nidvue-sdk/src/lib.rs` | done |
| T-0038 | `nidvue_config_peek`; honour `ui.start_minimised` and `ui.theme` | developer | P1 | T-0036 | `crates/nidvue-ffi/src/lib.rs`, `apps/nidvue-ui/main.cpp` | done |
| T-0039 | `ui-smoke` fails when the settings editor receives no rows | tester | P1 | T-0036 | `xtask/src/main.rs`, `apps/nidvue-ui/main.cpp` | done |
| T-0040 | An unreadable secret store degrades instead of aborting | developer | P0 | - | `crates/nidvue-jotta/src/auth.rs` | done |
| T-0041 | Release 0.2.1: version, tag, artefacts, release notes | release-engineer | P2 | T-0036..T-0040 | `Cargo.toml`, `CHANGELOG.md`, GitHub release | done |

Not closed, and why: the mount backends need a platform this environment does not
have; the UI frame budget needs a windowed session; `ui.language` needs shipped
translations. Each keeps its reason in [deviations](../../docs/deviations.md).

Two branches were written in parallel from the same base (`feat/ui-engine-completion`
and `feat/v0.2.0-close-deviations`); M8 lands second and reconciles them — one
dot-path implementation, one metrics exporter, one configuration lock, the
`Transfers` page in the new navigation, and the bench updated for the progress
callback. The reconciliation commit is `33fa2e4`.
Every one is recorded with its reason in [`../deviations.md`](../deviations.md).

## M6 - Team records and close (done)

Exit: `teamwork.py loop` reports STOP and `teamwork.py audit` reports zero problems.

The rows below are the close-out work items of this milestone, not task queue
records: the queues are empty and this milestone is executed by the CEO lane
directly, so it is planned here and evidenced in [Archive](Archive.md) and the
ledger rather than claimed in a queue.

| id | work item | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| W-1 | Committed team record set (Project, Plan, Board, Archive, Risks, Costs, Design, Research, Release, Features, Evolution, Architecture) | ceo | P2 | - | `docs/Team/` | done |
| W-2 | Architecture-conformance test for the crate contract | architect | P0 | - | `cargo xtask arch` | done |
| W-3 | Layered test records, one per area and layer, run by the owning role | ceo | P1 | - | `Tests.md`, `TestPlan.md` | done |
| W-4 | Ledger export committed with the record set | ceo | P1 | W-1 | `docs/Team/AuditManifest.json` | done |
| W-5 | Stale remote branches of merged pull requests deleted | release-engineer | P2 | - | `origin` | done |

## M11 - v0.3.0: Linux only, and start at login (in flight)

Exit: `cargo xtask ci` green on the merged revision; the tree names no macOS or
Windows build target; `nidvue-cli autostart enable` writes the XDG entry and the
Settings window toggles it.

The user's instruction was "focusing on linux host only right now, temporarily
drop the support to windows and macos for now". Both platforms were permanently
open deviations (no macOS runner, no WinFSP backend in `fuser`), so removing
them is a subtraction, not a deferral: the adapters, recipes, CI legs and
keyring backends are gone and recoverable from tag `v0.2.1`.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0001 | Drop macOS and Windows from every active surface | developer | P0 | - | `crates/nidvue-mount`, `Cargo.toml`, `deny.toml`, `.github/workflows/`, `packaging/`, `NOTICE` | selfcheck |
| T-0002 | Land XDG autostart end to end: core, C ABI, CLI, Settings toggle | developer | P0 | - | `crates/nidvue-core/src/autostart.rs`, `crates/nidvue-ffi`, `crates/nidvue-cli`, `apps/nidvue-ui` | selfcheck |
| T-0003 | Keyring probe raced itself; serialise it and key it per call | developer | P0 | - | `crates/nidvue-core/src/secret.rs` | selfcheck |
| T-0001 | Autostart integration test through the CLI | tester | P1 | developer:T-0002 | `crates/nidvue-cli/tests/autostart.rs` | selfcheck |
| T-0001 | Linux-only docs, CHANGELOG and 0.3.0 records | ceo | P2 | developer:T-0001, developer:T-0002, tester:T-0001 | `docs/`, `CHANGELOG.md`, `README.md`, `ARCHITECTURE.md`, `NOTICE` | done |
| T-0004 | The secret scan read any `sk-` as a key, so it flagged the path `xtask-ci.log` | developer | P0 | - | `xtask/src/main.rs` | done |
| T-0005 | Read `ui.language`: ship a German catalog, install the translator before the first frame, expose the setting, and honour `ui.theme` | developer | P0 | - | `apps/nidvue-ui/i18n/`, `main.cpp`, `SettingsPane.qml`, `CMakeLists.txt` | done |
| T-0006 | Measure the UI frame budget and put it in the gate | developer | P1 | - | `apps/nidvue-ui/main.cpp` (`--measure-frames`), `cargo xtask ui-frames` | done |
| T-0007 | The DCO step failed on the engine's own record commits | developer | P0 | - | `xtask/src/main.rs`, `docs/Team/Release.md` | done |

Exit state, stated: the gate is green (`cargo xtask ci`, 38 test binaries, ffi
smoke, ui smoke, ui frames, dco with the record-commit exemption printed); the six
criteria of the slice are verified; two engine defects stay open as `A-0001`
(record commits unsigned) and `A-0002` (`clean` deletes cited artifacts).

## M12 - v0.4.0: the five items that were listed as refusals or as unmeasurable

Exit: `cargo xtask ci` green on the merged revision; native plugins load behind a
trusted host; a password login stores no password; a tray-less desktop says so;
the 100k bench plans against a real remote listing; a live-account test exists and
refuses to pass without a token.

| id | task | role | prio | deps | artifact | status |
|---|---|---|---|---|---|---|
| T-0009 | Native plugin dylibs behind an explicit trusted host and an AGPL licence check | developer | P0 | - | `crates/nidvue-plugin`, `plugin.allow_native` | done |
| T-0010 | Tray best-effort on a desktop with no status-notifier host | developer | P1 | - | `Main.qml`, `SettingsPane.qml`, `ui-smoke` | done |
| T-0011 | Measure the 100k-path initial sync against a real remote listing | developer | P1 | - | `benches/benches/initial_sync.rs`, `Index::upsert_entries` | done |
| T-0012 | OIDC password login with no password stored | developer | P0 | - | `crates/nidvue-jotta/src/auth.rs`, `nidvue-cli login --username` | done |
| T-0002 | Live-account end-to-end test, gated on a token | tester | P1 | developer:T-0012 | `crates/nidvue-cli/tests/live.rs` | done |

Two of them produced a product fix the item did not ask for: the index needed a
batch writer (290 s → 76 ms for 100k rows), and the architecture contract had to
say what it means (only `ffi` exports C symbols; `plugin` declares a foreign ABI).

## Parallel rules

- Freeze interfaces first, then run dev / test / design lanes concurrently.
- Tests are written from the frozen contract, in parallel with implementation.
- Docs never block code or tests.
