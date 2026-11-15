# ARCHIVE - accepted results

Owner: CEO. Appended automatically when a task passes independent review. The
team's evidence ledger: what shipped, by whom, verified how, at which revision.
Gate order: owner self-check -> Reviewer verdict -> entry below.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

The nineteen entries below are the acceptance criteria of `v0.1.0`, each verified
by the command in its own line at the revision named. They are criteria records,
not task records: the delivered work predates the task queue (see [Plan](Plan.md)).

## AC-BUILD · cargo xtask ci exits 0 (ReleaseEngineer)
- date: 2026-11-14
- revision: `e73c89d`, re-run green at `35ba7a1` on 2026-11-15
- evidence: `cargo xtask ci` -> fmt --check, clippy -D warnings, test (156), doc,
  deny, spdx, secrets, dco, plugins, ui, ui-smoke -> `xtask ci: green`

## AC-LOGIN · Personal Login Token exchange, encrypted at rest, refresh-on-401 (Developer)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-jotta` -> 33 tests incl. refresh-on-401 and
  encrypted persistence

## AC-JFS · JFS XML codecs round-trip, unknown elements ignored, typed errors (Developer)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-jotta` -> liststream/entry/error codecs

## AC-UPLOAD · files/v1/allocate resume + MD5/size verification (Developer)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-jotta` -> allocate resume, dedup skip,
  size/MD5 verification

## AC-RATELIMIT · token bucket, jittered backoff, single-flight, AIMD (Developer)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-jotta` -> token bucket, jitter backoff, AIMD,
  request coalescing

## AC-CACHE · content-addressed store, index, LRU-K, hard cap (Developer)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-cache` -> 21 tests: blobs, index, LRU-K, hard
  cap, scrub

## AC-VFS · read-through, spooled writes, atomic downloads (Developer)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-vfs` -> 7 tests

## AC-SYNC · journal-first WAL, three-way plan, deterministic restart (Tester)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-sync` -> 25 tests incl. journal recovery

## AC-CONFLICT · a conflict keeps both versions, never a silent loss (Tester)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-sync` -> conflict copy + `ConflictRow` recorded

## AC-MOUNT · real FUSE3 mount serves the full op set (Tester)
- date: 2026-11-14
- revision: `e73c89d`, re-run at `35ba7a1` on 2026-11-15
- evidence: `cargo test -p nidvue-mount -- --include-ignored` -> 1 passed
  (write/read/rename/unlink through a real mount)

## AC-PLUGIN · wasmtime host with capabilities, fuel/memory limits, trap containment (Developer)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-plugin` -> 15 tests: 12 host tests and three example-plugin tests

## AC-SDK · documented API, examples, C header, JSON schema (Developer)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-sdk` -> 4 tests; `cargo doc` with
  `RUSTDOCFLAGS=-D warnings`

## AC-CLI · login/mount/sync/status/doctor, machine-readable status (Developer)
- date: 2026-11-14
- revision: `e73c89d`, re-run at `35ba7a1` on 2026-11-15
- evidence: `nidvue-cli --state <tmp> --json status` -> valid JSON, exit 0

## AC-UI · Qt6/QML app builds, launches, typed state over the C ABI (Developer)
- date: 2026-11-14
- revision: `e73c89d`, re-run at `35ba7a1` on 2026-11-15
- evidence: `cargo xtask ui && cargo xtask ui-smoke` -> `ui smoke: engine
  initialised, no QML errors`

## AC-PERF · p50 budgets measured, misses recorded with numbers (Tester)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo bench -p nidvue-benches` -> read 88 µs, readdir 290 µs,
  plan 1.44 µs/path; unmeasured budgets listed in `docs/deviations.md`

## AC-TESTS · unit + proptest + fake-Jotta + crash recovery all pass (Tester)
- date: 2026-11-14
- revision: `e73c89d`, re-run at `35ba7a1` on 2026-11-15
- evidence: `cargo test --workspace` -> 156 passed, 0 failed, 1 ignored

## AC-DOCS · README, ARCHITECTURE, ADRs, deviations match merged code (ProductManager)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `teamwork.py links` + a manual read of README/ARCHITECTURE/5 ADRs

## AC-PACKAGE · deb/tar.gz, dmg, MSI recipes with mount helper permissions (ReleaseEngineer)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `bash packaging/linux.sh` -> tarball + SHA256SUMS + LICENSE/NOTICE +
  source offer

## AC-INVARIANTS · the seven correctness invariants are executable (Tester)
- date: 2026-11-14
- revision: `e73c89d`
- evidence: `cargo test -p nidvue-invariants` -> 8 tests, one per invariant plus
  index persistence
