# ARCHITECTURE - design and frozen interfaces

Owner: Architect. The contract every lane builds against. Changing a frozen
interface needs a [DECISIONS](Decisions.md) entry plus notification to dependents.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

The full architecture — diagrams, the state machines, the durable state and the
reasoning behind each big call — lives in [`../../ARCHITECTURE.md`](../../ARCHITECTURE.md).
This page is the contract summary a lane reads before its first task; where the
two disagree, `ARCHITECTURE.md` wins and this page is the bug.

## 1. Context and constraints

- One Rust workspace, one engine, two front doors (CLI and Qt UI) over the same
  `Engine` facade.
- The Jottacloud API is undocumented and unsupported: it is isolated in
  `nidvue-jotta` so a breaking change is one crate, not the workspace.
- The kernel is touched only by `nidvue-mount`; the UI performs no I/O.
- AGPL-3.0-only; no paid infrastructure; no live account in CI.

## 2. Structure

```text
core  <-  jotta, cache, plugin  <-  vfs  <-  mount  <-  sync  <-  sdk  <-  ffi, cli
                                                                      <-  apps/nidvue-ui (C ABI)
```

A crate may depend on a lower layer, never on an equal or higher one. This is
executable: `cargo xtask arch` fails the build on a violation, and `cargo xtask
ci` runs it before clippy.

## 3. Frozen interfaces

| interface | exact signature / schema / CLI | errors | owner |
|---|---|---|---|
| `nidvue-sdk::Client` | `Client::open(config) -> Result<Client>`, `sync()`, `mount()`, `status()` | `NidvueError` | Architect |
| `nidvue-sdk` events | typed enum, serialised as the JSON schema in `crates/nidvue-sdk/schema/` | `NidvueError` | Architect |
| C ABI (`nidvue-ffi`) | `nidvue_*` functions, JSON in/out; every entry point catches panics | status codes, never a panic across the boundary | Architect |
| JFS XML codecs | `liststream`, entry, error codecs in `nidvue-jotta::jfs` | typed parse error | Architect |
| `nidvue-cli` | `login`, `logout`, `status`, `mount`, `sync`, `doctor`, `conflicts`, `plugins`; `--json` on the machine-readable ones | exit code + `NidvueError` | Architect |
| crate layering | the ranks in `xtask/src/main.rs::arch` | `cargo xtask arch` exit 1 | Architect |

## 4. Data model

- Blobs are content-addressed by SHA-256 under the cache root; the SQLite index
  (`index.sqlite`) is metadata only and is rebuildable from the blobs.
- The journal is a write-ahead log of intent, appended before any mutation; a
  replay at startup is deterministic and idempotent.
- A conflict produces a second file plus a `ConflictRow`; nothing is deleted to
  resolve one.
- Credentials live in `credentials.enc` (ChaCha20-Poly1305) with a `0600` key
  file beside it, or in `NIDVUE_SECRET_KEY`.

## 5. Runtime, deployment, security

- Linux: FUSE3 through `fuser` 0.18 (0.15 carries RUSTSEC-2021-0154).
- The Windows and macOS mount adapters were removed in `v0.3.0` rather than
  shipping an unbuildable backend ([`../deviations.md`](../deviations.md)).
- The mount never runs setuid; it uses the kernel's unprivileged FUSE path.
- Plugins are WASM under wasmtime with fuel, memory and wall-clock limits and no
  ambient authority.

## 6. Test seams

- Unit seams (Developer): per-crate `#[cfg(test)]` modules; the fake Jotta server
  in `crates/nidvue-jotta/tests/fake_jotta.rs` is the only network fake.
- Integration/E2E seams (Tester): `cargo test --workspace`, the real-mount test
  (`cargo test -p nidvue-mount -- --include-ignored`), the invariant crate, and
  `cargo xtask ui-smoke` for the UI.
- Architecture conformance (Architect): `cargo xtask arch`.
- Fakes and fixtures: the two example plugins under `plugins/`, a tempdir cache
  per test, and `wiremock` for HTTP shapes.

## 7. Module map

| module | path | owner | depends on | test seam |
|---|---|---|---|---|
| core | `crates/nidvue-core` | Developer | nothing Nidvue | unit |
| jotta | `crates/nidvue-jotta` | Developer | core | unit + fake server |
| cache | `crates/nidvue-cache` | Developer | core | unit |
| vfs | `crates/nidvue-vfs` | Developer | core, jotta, cache | unit |
| sync | `crates/nidvue-sync` | Developer | core, jotta, cache, vfs | unit + engine E2E |
| mount | `crates/nidvue-mount` | Developer | core, vfs | real FUSE mount |
| plugin | `crates/nidvue-plugin` | Developer | core | unit + example plugins |
| sdk | `crates/nidvue-sdk` | Developer | all engine crates | unit |
| ffi | `crates/nidvue-ffi` | Developer | core, sdk, mount | header test |
| cli | `crates/nidvue-cli` | Developer | core, cache, mount, plugin, sdk | `--json status` |
| ui | `apps/nidvue-ui` | Developer | the C ABI only | `cargo xtask ui-smoke` |

## Sources

- [`../../ARCHITECTURE.md`](../../ARCHITECTURE.md) — the authoritative document, in-repo.
- [`../../docs/adr`](../adr) — the five ADRs behind the big calls, in-repo.
