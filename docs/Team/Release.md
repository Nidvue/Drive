# RELEASE - build, ship, roll back

Owner: Release Engineer. The canonical answer to "how does this get built, shipped,
verified and reverted". Keep it executable: every command here must run as written.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## Build

| step | command | expected result |
|---|---|---|
| toolchain | `rustup show` (pinned by `rust-toolchain.toml`) | Rust 1.95 or newer |
| clean checkout | `git clone git@github.com:Nidvue/Drive.git && cd Drive` | a clean `main` |
| build | `cargo build --release` | engine, CLI and the C ABI library |
| UI build | `cargo xtask ui` | `apps/nidvue-ui/build/nidvue-ui` |
| package | `bash packaging/linux.sh` | `dist/nidvue-drive_<version>_<arch>.tar.gz`, `dist/SHA256SUMS` |
| macOS / Windows | `packaging/macos.sh`, `packaging/windows.ps1` | `.app`/`.dmg`, MSI/zip — recipe present, see [deviations](../deviations.md) 1 and 2 |
| provenance | `sha256sum -c dist/SHA256SUMS` | every artefact matches |

## CI gates

| gate | command | blocks a merge? |
|---|---|---|
| format | `cargo fmt --all -- --check` | yes |
| architecture contract | `cargo xtask arch` | yes |
| lints | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | yes |
| tests | `cargo test --workspace --all-features` | yes |
| docs | `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS=-D warnings` | yes |
| licences, advisories, bans, sources | `cargo deny check licenses bans sources advisories` | yes |
| SPDX headers | `cargo xtask spdx` | yes |
| secret scan | `cargo xtask secrets` | yes |
| DCO | `cargo xtask dco` | yes, on pull requests |
| UI | `cargo xtask ui && cargo xtask ui-smoke` | yes |
| C ABI | `cargo xtask ffi-smoke` (compile, link and run a C consumer) | yes |
| real FUSE mount | `cargo test -p nidvue-mount -- --include-ignored` | yes, in the `fuse mount smoke` job |
| CodeQL | `.github/workflows/codeql.yml` (C++ only) | alert, not a merge block |

`cargo xtask ci` runs all of the above except the CI-hosted CodeQL job.

## Deploy

- There is no server and no migration: the product is a client a user installs.
- Release path: a reviewed pull request merges into `main`, the Release Engineer
  tags `v<version>` (the CEO creates the tag), and the release workflow attaches
  the packaged artefacts and `SHA256SUMS` to the GitHub release.
- Configuration and secrets *mechanism* (never values): user credentials live in
  `<state>/credentials.enc` with a `0600` key file, or in `NIDVUE_SECRET_KEY`;
  CI uses the repository's own `GITHUB_TOKEN` with `contents: read`.
- Local state that survives an upgrade: `index.sqlite`, the blob store, the
  journal and the config. A schema change ships with a migration that runs at
  startup and is idempotent.

## Rollback

- One command to return to the previous version: `git checkout <previous-tag>`
  and rebuild, or reinstall the previous package. The state directory is not
  downgraded: a newer `index.sqlite` schema is refused with a typed error rather
  than read wrongly, and the index is rebuildable from the blobs.
- What is *not* reversible: a completed upload to Jottacloud. Deletions are
  journaled locally but the remote delete is real; the journal is the audit trail.
- Last drill: `bash packaging/linux.sh` at `35ba7a1` on 2026-11-15 — tarball and
  `SHA256SUMS` produced, `sha256sum -c` green.

## Observability

- Metrics: `nidvue-cli --json status` (health, index entries, cache bytes,
  conflicts, queue depth) and `nidvue-cli doctor --bench` for the budgets.
- Logs: structured, redacted, to the state directory; no telemetry is sent
  anywhere ([`../../SECURITY.md`](../../SECURITY.md)).
- Traces: `log.otel_endpoint` is a documented config field with no exporter wired
  ([deviations](../deviations.md) item 4).
- Alerts: none — there is no server to alert on. The tray icon is the user-facing
  health signal.

## Sources

- [`.github/workflows`](../../.github/workflows) — the workflow definitions, in-repo.
- [`../packaging.md`](../packaging.md) — mount helper permissions and package layout, in-repo.
