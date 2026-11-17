# Contributing to Nidvue Drive

## Before you start

- Read [`ARCHITECTURE.md`](ARCHITECTURE.md) and the relevant ADR in `docs/adr/`.
  The crate boundaries are load-bearing: a change that makes `sync` depend on
  Qt, or lets the UI do I/O, will be rejected regardless of how well it works.
- For anything larger than a bug fix, open an issue first. It is cheaper to
  disagree about a plan than about a diff.
- By contributing you agree to the [Code of Conduct](CODE_OF_CONDUCT.md) and to
  license your contribution under **AGPL-3.0-only**.

## Developer Certificate of Origin, no CLA

Every commit must carry a sign-off:

```sh
git commit -s -m "fix(sync): keep the conflict copy when the rename fails"
```

which appends:

```
Signed-off-by: Your Name <you@example.com>
```

CI rejects a commit without it (`cargo xtask dco`). There is no CLA: you keep
your copyright and grant the project the AGPL-3.0 licence.

## The gate

```sh
cargo xtask ci
```

must be green before a pull request is opened, and CI runs the same command on
Linux, the only supported platform in this release. It covers:

| Step | What it enforces |
|---|---|
| `cargo fmt --check` | one formatting, no debate |
| `cargo clippy -D warnings` | warnings are errors |
| `cargo test --workspace` | unit, property, integration and crash tests |
| `cargo doc -D warnings` | every public item documented, no broken links |
| `cargo deny check` | licence allowlist, advisories, no wildcard versions |
| `cargo xtask spdx` | SPDX header on every source file |
| `cargo xtask secrets` | no credential-shaped string in the tree |
| `cargo xtask dco` | every commit signed off |
| `cargo xtask ui` | the Qt/QML application builds |

## Style

- Rust 2021, `tokio` for async, `rayon` only for CPU-bound hashing and scanning.
- No `unwrap`/`expect` outside tests and provably infallible initialisation.
- No blocking call inside an async task; use `spawn_blocking` deliberately.
- Every public item has a doc comment. Every config value has a default here, a
  CLI flag and a UI control.
- Comments explain *why*, never *what*. Do not leave commented-out code or a
  `TODO` without a linked issue.
- Deliberate shortcuts are marked `ponytail:` with the ceiling and the upgrade
  path, e.g. `// ponytail: global lock; per-pair locks if throughput matters`.

## Commits and pull requests

- Conventional commits: `feat:`, `fix:`, `perf:`, `docs:`, `refactor:`,
  `test:`, `chore:`. The scope is the crate: `fix(sync): …`.
- Small, reviewable slices. Every commit leaves `cargo xtask ci` green.
- A pull request states: what changed, why, how it was verified (the command and
  the result), and what it deliberately does not do.

### `main` is protected: branch, pull request, rebase

Direct pushes to `main` are rejected for everyone, administrators included. The
only path is:

```sh
git switch -c feat/my-change
git commit -s -m "feat(sync): …"
git push -u origin feat/my-change
gh pr create --fill
# CI must be green on the pull request's head
gh pr merge --rebase --delete-branch
```

The repository allows **rebase merges only** (no merge commits, no squash), so
the history on `main` is linear and every commit keeps its DCO sign-off. A pull
request must be up to date with `main` before it merges; rebase your branch
rather than merging `main` into it. Head branches are deleted on merge.

Required on `main`: a pull request, a green `ci` run (rust on Linux,
licences/advisories/SPDX/secrets, the Qt build and smoke test, the FUSE mount
test), resolved conversations, no force-push, no branch deletion. The Release
Engineer owns these settings and re-reads them from the host API
(`docs/Remotes.md`), because an unprotected setting is a defect on par with a red
build.

If a release needs a hotfix, it takes the same path: a branch, a pull request, a
rebase merge. There is no admin bypass and no "just this once".

## Adding a dependency

1. It must pass `deny.toml`. GPL-3.0 is acceptable (one-way compatible into
   AGPL-3.0); GPL-2.0-only, SSPL, BUSL, Elastic and non-commercial licences are
   not.
2. Add it to `docs/licensing.md` with its licence and how it is linked.
3. Justify it in the pull request: what the standard library or an existing
   dependency fails to do.

## Adding a platform backend

A mount backend is one thin adapter over `nidvue-vfs::Vfs`. Do not put policy in
it. Document any semantic difference in `docs/vfs-semantics.md` and add a
skipped-gracefully test in CI for platforms where a mount is not available.
