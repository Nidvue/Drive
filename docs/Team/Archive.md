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

## T-0001 · OS keyring SecretStore backend with encrypted-file fallback  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: developer-1)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Re-read the diff: KeyringStore maps NoEntry to Ok(None) so a fresh install and a lost entry are indistinguishable to the caller; open_store(Auto) probes with a throwaway entry, stores, reads back and deletes before choosing, and on any platform error returns the file store with the platform's own message in fallback_reason; Keyring/File never probe so an explicit choice cannot silently downgrade; the file store's 0600 and ChaCha20 behaviour is unchanged. Boundary cases covered by tests: no entry, wrong key (fails closed), empty backend name, fallback describe() text. Scope is the secret module plus the config enum; nothing else changed. | test: cargo test -p nidvue-core --all-features secret @798a123 -> 7 passed, 0 failed, 1 ignored | artifacts: crates/nidvue-core/src/secret.rs:0f3cba522f5fd1bd:17061 | replay: cargo test -p nidvue-core --all-features secret | preflight: PF-19cd185139d4 @- (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: cargo test -p nidvue-core --all-features secret @798a123 -> 7 passed, 0 failed, 1 ignored; nidvue-cli --state <tmp> doctor @798a123 -> 'secret: file (fell back from keyring: ... DBus error: The name is not activatable)' | artifacts: crates/nidvue-core/src/secret.rs:0f3cba522f5fd1bd:17061 | replay: cargo test -p nidvue-core --all-features secret
- findings history: 
- branch / merge: - / pending

## T-0002 · Prometheus /metrics endpoint on log.prometheus_bind  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer-1)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Re-read the diff: the listener is non-blocking with a 20 ms accept poll so Drop can join the thread and release the port (dropping_the_server_frees_the_port proves it); one request per connection, 2 s read and write timeouts so a slow client cannot pin a thread; the query string is stripped before matching; an unknown path is 404 and /healthz is separate from /metrics; the body is rendered from the same MetricsCache the OTLP exporter reads, so the two cannot disagree; Content-Length is the byte length of the rendered body. Concurrency: one thread, sequential accept, no shared mutable state beyond the RwLock read. Scope: the telemetry crate only. | test: cargo test -p nidvue-telemetry -- scrape @fce943a -> 2 passed, 0 failed | artifacts: crates/nidvue-telemetry/src/lib.rs:c337fb15fe8947c2:13747 | replay: cargo test -p nidvue-telemetry -- scrape | preflight: PF-19cd185139d4 @- (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: cargo test -p nidvue-telemetry -- scrape @fce943a -> 2 passed, 0 failed; nidvue-cli --state <tmp> doctor with log.prometheus_bind=127.0.0.1:0 @fce943a -> 'metrics: scrape http://127.0.0.1:34803/metrics' | artifacts: crates/nidvue-telemetry/src/lib.rs:c337fb15fe8947c2:13747 | replay: cargo test -p nidvue-telemetry -- scrape
- findings history: 
- branch / merge: - / pending

## T-0003 · OTLP/JSON metrics exporter on log.otel_endpoint  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer-1)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Re-read the diff: the endpoint is normalised once at construction so a trailing slash or an already-complete /v1/metrics cannot produce a double path; an empty snapshot is not exported (nothing to say); a non-2xx answer and a dead collector are both Err and the spawn loop only logs, so telemetry can never fail a sync; timeUnixNano is a string because OTLP/JSON requires uint64 as a string; the payload is core::metrics, pure and unit-tested, so the transport cannot change a value. Scope: the telemetry crate plus the SDK start_otlp wiring. | test: cargo test -p nidvue-telemetry -- exporter @fce943a -> 2 passed, 0 failed | artifacts: crates/nidvue-telemetry/src/lib.rs:c337fb15fe8947c2:13747 | replay: cargo test -p nidvue-telemetry -- exporter | preflight: PF-19cd185139d4 @- (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: cargo test -p nidvue-telemetry -- exporter @fce943a -> 2 passed, 0 failed; cargo test -p nidvue-sdk --test telemetry @fce943a -> 7 passed (the scrape and the status agree on every value) | artifacts: crates/nidvue-telemetry/src/lib.rs:c337fb15fe8947c2:13747 | replay: cargo test -p nidvue-telemetry -- exporter
- findings history: 
- branch / merge: - / pending

## T-0004 · config get/set/list/path CLI with full key coverage  (developer)
- date: 2026-11-16
- priority: P1
- owner: developer role (work by: Developer-1)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Re-read the diff: get/set walk the serde_json projection of AppConfig, so the key set is the file's key set by construction (every_leaf_is_readable_by_its_own_path walks all of them); set coerces against the type already present and then runs validate(), so a value that would break a cross-field invariant is refused before the file is written (an_invalid_result_is_refused_by_validate); null clears an Option and fails on a required field rather than silently writing the string; an unknown key suggests the nearest path by edit distance. Empty and duplicate cases: an empty path returns the whole tree only for walk, an empty string is a value, and a repeated set is idempotent. Scope: the cli lib module plus the config subcommand. | test: cargo test -p nidvue-cli @fce943a -> 14 passed, 0 failed (9 unit, 5 integration) | artifacts: crates/nidvue-cli/src/lib.rs:2c05e1670e8bb1d6:13223 | replay: cargo test -p nidvue-cli | preflight: PF-19cd185139d4 @- (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: cargo test -p nidvue-cli @fce943a -> 14 passed, 0 failed; nidvue-cli --state <tmp> config set/get/list/path @fce943a -> set and get agree, an unknown key exits 1 with a suggestion, config list --json covers every key | artifacts: crates/nidvue-cli/src/lib.rs:2c05e1670e8bb1d6:13223 | replay: cargo test -p nidvue-cli
- findings history: 
- branch / merge: - / pending

## T-0005 · Transfer registry, Status.transfers, FFI accessor and a virtualised Transfers view  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer-1)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Re-read the diff: the registry is keyed by direction and path because one worker owns a path, so a second event for the same transfer updates rather than duplicates (the_two_directions_are_separate_rows covers the collision case); the list is capped at MAX_TRANSFERS and eviction skips live rows, so a running transfer is never dropped (the_list_is_bounded_and_keeps_live_rows); a poisoned lock returns the inner data instead of panicking, so a panic elsewhere cannot wedge status; the feeder task ignores Lagged rather than dying; Status carries transfer_limit so the UI does not hard-code 512. Second-call case: TransferDone after TransferQueued is an update, and clear() then record() starts clean. Scope: core/transfers, the SDK accessors, the FFI pair, Transfers.qml and the bridge property. | test: cargo test -p nidvue-sdk --test telemetry -- transfers @fce943a -> 1 passed, 0 failed | artifacts: crates/nidvue-core/src/transfers.rs:37419ca5dc8229b6:10789 | replay: cargo test -p nidvue-sdk --test telemetry -- transfers | preflight: PF-19cd185139d4 @- (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: cargo test -p nidvue-sdk --test telemetry @fce943a -> 7 passed; cargo xtask ffi-smoke @fce943a -> the C consumer reads transfers_json as [] and clears it; cargo xtask ui-smoke @fce943a -> engine initialised, no QML errors with the Transfers page in the stack | artifacts: crates/nidvue-core/src/transfers.rs:37419ca5dc8229b6:10789 | replay: cargo test -p nidvue-sdk --test telemetry
- findings history: 
- branch / merge: - / pending

## T-0001 · Measure cold-read and sustained-transfer budgets against the fake server  (tester)
- date: 2026-11-16
- priority: P1
- owner: tester role (work by: Tester-1)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Re-read the bench: the emulated link is the bench's own input and is printed, so 'percent of capacity' is measured against a known rate, not a nominal one; the cold read uses a 1 MiB object because the VFS fetches a whole object atomically, so a 1 MiB read of an 8 MiB file would measure the object, not the budget (the first version of this bench made exactly that mistake and reported a 1.49 s MISS); the sustained case uses 64 distinct 128 KiB objects so every byte crosses the link and per-object overhead is inside the number; the warm read reuses the cold object to prove the cache, not the link. Peak RSS comes from VmHWM so it is the high-water mark, not the final value. Honest limit stated in the output: the UI frame budget is not measurable here and is not claimed. | test: cargo bench -p nidvue-benches --bench transfer @5514f75 -> cold 189 ms PASS, sustained 86.8 % PASS, warm 274x PASS, peak rss 8.1 MiB | artifacts: benches/benches/transfer.rs:aee32f404862236e:7165 | replay: cargo bench -p nidvue-benches --bench transfer | preflight: PF-19cd185139d4 @- (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: cargo bench -p nidvue-benches --bench transfer @5514f75 -> cold read 1 MiB 189 ms (budget 400 ms) PASS; sustained 8 MiB in 64 objects 5.18 MiB/s = 86.8 % of a 50 Mbit/s emulated link (budget 80 %) PASS; warm 690 us = 274x PASS | artifacts: benches/benches/transfer.rs:aee32f404862236e:7165 | replay: cargo bench -p nidvue-benches --bench transfer
- findings history: 
- branch / merge: - / pending

## T-0002 · Measure a 100k-path initial sync plan and apply  (tester)
- date: 2026-11-16
- priority: P1
- owner: tester role (work by: Tester-1)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Re-read the bench: it measures scan, index and plan, and the fixture build time is printed separately so it is not counted as engine work; the expected path count includes the 100 directories the scanner legitimately returns, which is the bug the first run caught (100100 vs 100000); the plan is over an empty remote so all 100100 paths are creates; peak RSS is VmHWM. The claim is deliberately narrow: this is not 100k network transfers, and docs/deviations.md item 6 says so. | test: cargo bench -p nidvue-benches --bench initial_sync @5514f75 -> scan 664 ms, index 861 ms, plan 57 ms, peak rss 54 MiB (566 B/path) | artifacts: benches/benches/initial_sync.rs:80406b5c9692583a:4191 | replay: cargo bench -p nidvue-benches --bench initial_sync | preflight: PF-19cd185139d4 @- (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: cargo bench -p nidvue-benches --bench initial_sync @5514f75 -> 100000 paths scanned in 664 ms, indexed in 861 ms, planned in 57 ms, peak rss 54 MiB | artifacts: benches/benches/initial_sync.rs:80406b5c9692583a:4191 | replay: cargo bench -p nidvue-benches --bench initial_sync
- findings history: 
- branch / merge: - / pending

## T-0001 · v0.2.0 version bump, changelog, release notes and packaging drill  (release-engineer)
- date: 2026-11-16
- priority: P2
- owner: release-engineer role (work by: ReleaseEngineer)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Re-read the change: the workspace version is 0.2.0 and every path dependency matches it, so a published crate cannot resolve a 0.1.0 sibling; CHANGELOG follows Keep a Changelog with Added/Changed/Fixed under 0.2.0 and leaves Unreleased empty; the release doc's build, gate and rollback commands still match the tree (xtask ci now includes ffi-smoke). Not done and stated as such: no tag, no push, no published artefact — tagging and publishing are the CEO's step and the Reviewer's pass is a precondition. | test: cargo xtask ci @a08a4b1 -> green (fmt, arch, clippy -D warnings, tests 197 passed, doc, deny, spdx, secrets, dco, plugins, ffi-smoke, ui, ui-smoke) | artifacts: CHANGELOG.md:95b356ea85faeec0:3900 | replay: cargo xtask ci | preflight: PF-19cd185139d4 @- (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: cargo xtask ci @a08a4b1 -> 'xtask ci: green'; grep -c '0.2.0' Cargo.toml @a08a4b1 -> version and all nine path dependencies at 0.2.0; CHANGELOG.md @a08a4b1 -> 0.2.0 section present, Unreleased empty | artifacts: CHANGELOG.md:95b356ea85faeec0:3900 | replay: cargo xtask ci
- findings history: 
- branch / merge: - / pending

## T-0001 · Update deviations, ARCHITECTURE, README and configuration for v0.2.0  (product-manager)
- date: 2026-11-16
- priority: P2
- owner: product-manager role (work by: ProductManager)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Re-read the docs against the tree: every deviation that closed cites the test or the bench that proves it, and every one that stayed open now carries a checkable reason (fuser 0.18's feature list has no WinFSP; the keyring test is #[ignore]d because this machine has a D-Bus bus and no provider); docs/configuration.md lists the three new keys with their defaults read from config.rs, plus the dot-path commands that were run; ARCHITECTURE.md's crate graph matches cargo xtask arch at 11 crates and the two-egress rule matches the code in xtask; docs/performance.md quotes only commands that were executed, with the emulated-link caveat stated in the same paragraph as the number; README status matches CHANGELOG. Nothing claims a number that no command produced. | test: cargo xtask arch @9c34e07 -> + arch: crate contract holds (11 crates); teamwork links -> every relative link resolves | artifacts: docs/deviations.md:f8c3474dd1ea3071:6091 | replay: cargo xtask arch | preflight: PF-19cd185139d4 @- (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: cargo xtask arch @9c34e07 -> 11 crates, contract holds; grep for the three new config keys in docs/configuration.md @9c34e07 -> present with defaults matching nidvue-core/src/config.rs; docs/performance.md @9c34e07 -> every quoted number matches the bench output recorded in this review's siblings (189 ms, 86.8 %, 274x, 664/861/57 ms, 54 MiB) | artifacts: docs/deviations.md:f8c3474dd1ea3071:6091 | replay: cargo xtask arch
- findings history: 
- branch / merge: - / pending
