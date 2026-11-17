# RELEASE - build, ship, roll back

Owner: Release Engineer. The canonical answer to "how does this get built, shipped,
verified and reverted". Keep it executable: every command here must run as written.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md) · [deviations](../deviations.md) · [packaging](../packaging.md)

## Build

| step | command | expected result |
|---|---|---|
| toolchain | `rustup show` (pinned by `rust-toolchain.toml`) | Rust 1.95 or newer |
| clean checkout | `git clone git@github.com:Nidvue/Drive.git && cd Drive` | a clean `main` |
| build | `cargo build --release` | engine, CLI and the C ABI library |
| UI build | `cargo xtask ui` | `apps/nidvue-ui/build/nidvue-ui` |
| package | `bash packaging/linux.sh` | `dist/nidvue-drive_<version>_<arch>.tar.gz`, `dist/SHA256SUMS` |
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
| UI frame budget | `cargo xtask ui-frames` (p99 ≤ 16 ms) | yes |
| C ABI | `cargo xtask ffi-smoke` (compile, link and run a C consumer) | yes |
| real FUSE mount | `cargo test -p nidvue-mount -- --include-ignored` | yes, in the `fuse mount smoke` job |
| CodeQL | `.github/workflows/codeql.yml` (C++ only) | alert, not a merge block |

`cargo xtask ci` runs all of the above except the CI-hosted CodeQL job. The
forge runs the same steps as separate required jobs (`rust (ubuntu-24.04)`,
`licences, advisories, spdx, secrets`, `qt ui`, `fuse mount smoke`); the dco job
calls `cargo run -p xtask -- dco` so the trailer rule and its one exemption live
in one place.

**The one exemption, stated.** The teamwork engine renders `docs/Team/Archive.md`
and `docs/Team/Board.md` after every verdict and commits them itself, without a
`Signed-off-by` trailer (tracked as `A-0001`; the engine should sign its own
commits). `cargo xtask dco` therefore exempts a commit only when **both** hold:
its subject starts with `chore(records):` **and** every path it touched is under
`docs/Team/`. The exemption prints its count, so a run that leans on it says so.
No authored commit is exempt: a change to code, CI, packaging or the docs outside
`docs/Team/` still needs the trailer, and naming a commit `chore(records):` while
touching code fails the check. Verified by
`xtask::tests::only_a_records_commit_that_touches_records_is_exempt`.

## Branch protection

`main` is protected on the forge and the Reviewer is the only approver. The
settings below are read back from the API, never assumed:

```sh
gh api repos/Nidvue/Drive/branches/main/protection
```

| setting | value | why |
|---|---|---|
| pull request required | yes | no direct push to `main`; every change is a pull request |
| required status checks (strict) | `rust (ubuntu-24.04)`, `licences, advisories, spdx, secrets`, `qt ui`, `fuse mount smoke` | green on the merged revision, and the branch must be up to date with `main` |
| required approvals | 0 - see the constraint below | one forge identity cannot approve its own pull request |
| dismiss stale approvals | yes | a new push invalidates the earlier approval |
| approval of the last push | off - see the constraint below | measured: it blocks every pull request while one identity exists |
| code-owner review | off until a second identity exists | `.github/CODEOWNERS` names the reviewer on every path |
| force-push, deletion | refused | a rewrite or a deletion of `main` is a red line |
| linear history | required | no merge commits on `main` |
| conversation resolution | required | a review thread must be answered before the merge |
| enforce for admins | yes | the owner has no bypass |

**Who may merge.** Only the Reviewer, and never the author of the change. The
engine enforces that rule, not the forge:

```sh
python3 scripts/teamwork.py pr merge --pr <id> --by reviewer-1
```

`pr merge` refuses a role that is not the Reviewer and refuses the PR's own author;
the review verdict, the merge and the merge commit are ledger records.

**The one gap and its switch.** `required_approving_review_count` stays `0` and
`require_last_push_approval` stays off, because the repository has exactly one
forge identity (`VisionPZ`, the organisation owner). GitHub forbids approving your
own pull request, so a count of `1` would make every pull request unmergeable; and
`require_last_push_approval` alone blocks without any approval being possible -
measured on pull request 20, which reported `mergeStateStatus: BLOCKED` with every
required check green until that flag was turned off, and `CLEAN` immediately
after. The moment a second identity exists - a
`reviewers` team with a second member, or a bot account - set the count to `1` and
turn on `require_code_owner_reviews`; `.github/CODEOWNERS` already names the
reviewer, so nothing else changes. Creating that second account is an external,
irreversible step and needs the owner's authorization.

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

## Released versions

| version | tag | revision | artefacts | evidence |
|---|---|---|---|---|
| 0.5.0 | `v0.5.0` | `25066e3` | `nidvue-drive-0.5.0-linux-amd64.tar.gz`, `nidvue-drive_0.5.0_amd64.deb`, `SHA256SUMS`, `SOURCE-OFFER.txt` (GitHub release) | local gate green at `3f2626b` (41 test binaries, ffi smoke, ui smoke, `ui frames` p99 0.327 ms, dco clean); CI run 44100706701 green on that revision; **live account**: `sync run` -> applied 2 failed 0 with both files on the account and `sync watch` uploading a new file with no explicit run; release run 44102239805 -> 9m15s, green |
| 0.4.0 | `v0.4.0` | `f6910b0` | `nidvue-drive-0.4.0-linux-amd64.tar.gz`, `nidvue-drive_0.4.0_amd64.deb`, `SHA256SUMS`, `SOURCE-OFFER.txt` (GitHub release) | local gate green at `02c4581` (`cargo xtask ci`: 41 test binaries, ffi smoke, ui smoke with 49 settings rows, `ui frames` p99 0.252 ms, native plugin tests, dco with its record-commit exemption printed); CI run 44086468345 green on `02c4581` (rust ubuntu-24.04, licences/advisories/spdx/secrets, qt ui, fuse mount smoke, links); release run 44087779333 -> build 10m16s, release 13s, both green |
| 0.3.0 | `v0.3.0` | `ca968fd` | `nidvue-drive-0.3.0-linux-amd64.tar.gz`, `nidvue-drive_0.3.0_amd64.deb`, `SHA256SUMS`, `SOURCE-OFFER.txt` (GitHub release) | local gate green at `bd8ca01` (`cargo xtask ci`: 38 test binaries, ffi smoke, ui smoke, `ui frames` p99 0.295 ms, dco with its record-commit exemption printed); CI run 44073124942 green on `bd8ca01` (rust ubuntu-24.04, licences/advisories/spdx/secrets, qt ui, fuse mount smoke, links); release run 44076147917 -> build 8m37s, release 25s, both green; packaged tarball smoke-tested: `nidvue-cli --version` -> `nidvue-cli 0.3.0`, `autostart --help` lists status/enable/disable |
| 0.2.1 | `v0.2.1` | `7d1c7ad` | `nidvue-drive-0.2.1-linux-amd64.tar.gz`, `SHA256SUMS` (GitHub release) | `cargo xtask ci` green at the tagged revision (231 tests); CI run 43902102553 green on `main` at the tagged revision; `ui-smoke` -> `48 settings rows`; packaged binaries smoke-tested: `nidvue-cli --version` -> 0.2.1, `config set/get cache.max_size_bytes` -> 1048576, UI `--smoke-windows` -> exit 0 |
| 0.2.0 | `v0.2.0` | `6b42079` | `nidvue-drive-0.2.0-linux-amd64.tar.gz`, `SHA256SUMS` (GitHub release) | `cargo xtask ci` green at the tagged revision; CI run 43879186747 green (rust ubuntu + windows, licences/advisories/spdx/secrets, qt ui, fuse mount smoke); packaged binaries smoke-tested from the extracted tarball: `nidvue-cli --version` -> `nidvue-cli 0.2.0`, `config get cache.max_size_bytes` -> `21474836480`, UI `--smoke-windows` -> exit 0 |
| 0.1.0 | `v0.1.0` | `0f24d5f` | none republished | tagged retroactively at the 0.2.0 release because the docs referenced `v0.1.0` and no tag existed; the commit is the last one whose `Cargo.toml` says 0.1.0 |

`dpkg-deb` is absent from this local build environment, so a local
`bash packaging/linux.sh` skips the `.deb` and produces the relocatable tarball;
the 0.3.0 release ran on an Ubuntu runner, where `dpkg-deb` exists, and the release
carries both the `.deb` and the tarball.

## Repository integrity (re-read after the 0.2.0 release)

Read back from the host, not from a file in the tree:

```sh
gh api repos/Nidvue/Drive/branches/main/protection
```

| setting | value | why |
|---|---|---|
| required status checks | `rust (ubuntu-24.04)`, `licences, advisories, spdx, secrets`, `qt ui`, `fuse mount smoke` | the four jobs that gate the product; `rust (windows-2022)` left the list in `v0.3.0` with the platform |
| strict | true | a branch must be rebased on the current `main` before it merges |
| required approvals | 0 | the repository has one collaborator who authors every pull request, so a required approval is a deadlock rather than a gate; the review that applies is the recorded verdict plus these checks ([Decisions](Decisions.md) D-20261116-CEO-7) |
| enforce admins | true | the owner cannot bypass the checks either |
| force pushes / deletions | false / false | history is append-only |
| linear history | true | rebase-merge only |
| conversation resolution | true | a review finding cannot be left unanswered |

Restoring `required_approving_review_count: 1` is the correct change the day a
second collaborator exists.

## Observability

- Metrics: `nidvue-cli --json status` (health, index entries, cache bytes,
  conflicts, queue depth) and `nidvue-cli doctor --bench` for the budgets.
- Logs: structured, redacted, to the state directory; no telemetry is sent
  anywhere ([`../../SECURITY.md`](../../SECURITY.md)).
- Traces: `log.otel_endpoint` exports OTLP/HTTP JSON metrics to `/v1/metrics`
  every `log.otel_interval_s` seconds and one span per finished pass, failed
  transfer and error-health transition to `/v1/traces`; `log.prometheus_bind`
  serves `/metrics` and `/healthz`. A collector that is down is a logged warning,
  never a sync failure.
- Alerts: none — there is no server to alert on. The tray icon is the user-facing
  health signal.

## Sources

- [`.github/workflows`](../../.github/workflows) — the workflow definitions, in-repo.
- [`../packaging.md`](../packaging.md) — mount helper permissions and package layout, in-repo.
