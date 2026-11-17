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

## T-0025 · Dot-path configuration for the CLI, the SDK and the C ABI  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1 (sequential; see Board snapshot)
- task / issue: -
- area: config
- self-check: [pass] Re-read the change: `nidvue_core::config::dot` walks the serialised `AppConfig`, so a key in the file is a key on the path and an unknown key is refused with the closest match; a list segment is an index or the `name` of an entry, so `sync.pairs.docs.exclude` addresses one pair without counting; `set` coerces against the type already there and validates before the value replaces the old one, so a refused value leaves the file byte-identical; the CLI, the SDK (`config_get`/`config_set`) and the C ABI (`nidvue_client_config_get`/`_set`) all call the one implementation, and the old duplicate in `nidvue-cli` is now a re-export. | test: cargo test -p nidvue-core -p nidvue-cli -p nidvue-sdk @33fa2e4 -> 40 passed, 0 failed | artifacts: crates/nidvue-core/src/config.rs | replay: cargo test -p nidvue-core config
- author evidence: cargo test -p nidvue-cli @33fa2e4 -> 3 cli + 5 config tests pass, including a pair-name path and a rejected zero cache
- reviewer evidence: cargo test -p nidvue-core config @33fa2e4 -> 7 passed (path_get_and_set_reach_every_value, path_set_refuses_unknown_keys_and_bad_values, leaves_name_each_pair_and_its_values) | replay: cargo test -p nidvue-core config
- findings history: the first pass duplicated the CLI's dot-path module in core; the reviewer required one implementation, so the CLI re-exports core's module
- branch / merge: feat/ux-parity-v2 / pending

## T-0028 · The Conflict Center resolves a conflict  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1 (sequential)
- task / issue: -
- area: conflicts
- self-check: [pass] Re-read the change: `Conflicts.qml` and `nidvue-cli conflicts resolve` both pass `keep_local`/`keep_remote`, and `ConflictPolicy::parse` accepted neither, so every resolve click returned a config error. The parser now accepts both spellings (with the hyphenated variants) and the config names, and a test covers every documented spelling plus a nonsense one. Nothing else changed: the resolution path, the conflict copy and the archive entry are untouched. | test: cargo test -p nidvue-core conflict_policy @33fa2e4 -> 1 passed | artifacts: crates/nidvue-core/src/types.rs | replay: cargo test -p nidvue-core conflict_policy
- author evidence: cargo test -p nidvue-core @33fa2e4 -> conflict_policy_parses_every_documented_spelling passes; the UI buttons pass `keep_local`, which now parses to `PreferLocal`
- reviewer evidence: grep -n 'keep_local' apps/nidvue-ui/qml/Conflicts.qml @33fa2e4 -> the button passes the label the parser now accepts; cargo test -p nidvue-core conflict_policy @33fa2e4 -> 1 passed
- findings history: -
- branch / merge: feat/ux-parity-v2 / pending

## T-0026 · Sync-pair management and the long tail from the CLI  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1 (sequential)
- task / issue: -
- area: cli
- self-check: [pass] Re-read the change: `sync add` gained `--include`, `--exclude`, `--conflict-policy` and `--paused`; `sync remove`, `sync pause`, `sync resume` and `sync list --json` were added, each persisting through `AppConfig::save` (so `validate` runs before the write) and each reporting a missing pair as an error instead of a silent no-op. `restore_sigpipe` makes `nidvue-cli … | head` end the output instead of panicking. The empty case (`sync remove ghost`) and the second call (`sync add` twice) are covered by tests. | test: cargo test -p nidvue-cli @33fa2e4 -> 8 passed, 0 failed | artifacts: crates/nidvue-cli/src/main.rs, crates/nidvue-cli/tests/cli.rs | replay: cargo test -p nidvue-cli
- author evidence: cargo test -p nidvue-cli @33fa2e4 -> a_pair_is_added_listed_filtered_and_removed, config_get_set_reach_a_nested_value_and_reject_a_bad_one, config_list_names_every_key_it_can_set all pass
- reviewer evidence: nidvue-cli --state <tmp> sync add docs … --exclude '**/node_modules' then --json sync list @33fa2e4 -> exclude array present; nidvue-cli config list | head -> no panic
- findings history: -
- branch / merge: feat/ux-parity-v2 / pending

## T-0027 · Pair filters, log filter and tray notifications in the UI  (developer)
- date: 2026-11-16
- priority: P1
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1 (sequential)
- task / issue: -
- area: ui
- self-check: [pass] Re-read the change: the Settings pair row shows the pair's exclude globs and writes them back through `nidvue_client_sync_pair_set_filters` (comma-separated, trimmed, empty clears), which is validated in Rust before the write; the Activity page filters its list without touching the model; the tray compares the conflict count and the health between refreshes, so a new conflict and the first error of a run are announced once and a startup is silent. Empty case: no pairs, no filter match, and no tray host each degrade to the previous behaviour. | test: cargo xtask ui-smoke @33fa2e4 -> engine initialised, no QML errors | artifacts: apps/nidvue-ui/qml/SettingsPane.qml, Logs.qml, tray.cpp | replay: cargo xtask ui && cargo xtask ui-smoke
- author evidence: cargo xtask ui && cargo xtask ui-smoke @33fa2e4 -> UI built, both windows instantiate offscreen with no QML errors
- reviewer evidence: cargo xtask ui-smoke @33fa2e4 -> "engine initialised, no QML errors"; grep -n setSyncPairFilters apps/nidvue-ui/bridge.cpp -> worker slot, bridge method and connection present
- findings history: -
- branch / merge: feat/ux-parity-v2 / pending

## T-0029 · One configuration lock in the SDK  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1 (sequential)
- task / issue: -
- area: sdk
- self-check: [pass] Re-read the change: `Inner::config` is an `RwLock<AppConfig>`, every writer (`set_paused`, `set_bandwidth_limit_bps`, `set_cache_limit_bytes`, the pair operations, `config_set`) goes through one path that validates, saves atomically and then applies the live atomics, so `Client::config()` can no longer return a value older than the last write. Concurrent writers serialise on the lock; readers clone. A rejected value leaves both the file and the in-memory copy untouched, which the test asserts. | test: cargo test -p nidvue-sdk @33fa2e4 -> 5 lib tests pass | artifacts: crates/nidvue-sdk/src/lib.rs | replay: cargo test -p nidvue-sdk
- author evidence: cargo test -p nidvue-sdk config_set_persists_and_applies_live_values @33fa2e4 -> 1 passed
- reviewer evidence: cargo test -p nidvue-sdk pair_filters_are_validated_and_persisted @33fa2e4 -> 1 passed; cargo test -p nidvue-sdk @33fa2e4 -> 5 passed
- findings history: -
- branch / merge: feat/ux-parity-v2 / pending

## T-0030 · CLI integration tests  (tester)
- date: 2026-11-16
- priority: P1
- owner: tester role (work by: Tester)
- reviewer: Reviewer-1 (sequential)
- task / issue: -
- area: cli
- self-check: [pass] Re-read the change: the tests run the built binary against a temporary state directory, so they cover argument parsing, the file the CLI writes and the value a later read returns — the seam that actually breaks. They assert the failure cases too (unknown key, a zero cache, a missing pair) and that a refused write leaves the previous value in place. | test: cargo test -p nidvue-cli --test cli @33fa2e4 -> 3 passed | artifacts: crates/nidvue-cli/tests/cli.rs | replay: cargo test -p nidvue-cli --test cli
- author evidence: cargo test -p nidvue-cli --test cli @33fa2e4 -> 3 passed, 0 failed
- reviewer evidence: cargo test -p nidvue-cli --test cli @33fa2e4 -> 3 passed; the suite fails when `config set` stops writing the file (checked by reading the assertion bodies, not by mutation)
- findings history: -
- branch / merge: feat/ux-parity-v2 / pending

## T-0036 · The whole configuration is editable in the window  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1 (recorded verdict on pull request 16)
- task / issue: -
- area: ui
- self-check: [pass] Re-read the change: the card renders `Bridge.configRows`, which comes from `nidvue_client_config_list_json` (the flat `dot.path -> value` map the CLI's `config list --json` prints), filtered by a search box; each row writes through `Bridge.setConfigValue` -> `nidvue_client_config_set`, so the validation that refuses a zero cache in the CLI refuses it here too. Empty case: no engine -> no rows, and the card shows nothing rather than a stale list; a rejected value leaves the row's text and the file as they were. Scope: one card, one FFI read, one FFI write; no other page touched. | test: cargo xtask ui-smoke @7d1c7ad -> "+ ui smoke: engine initialised, 48 settings rows, no QML errors" | artifacts: apps/nidvue-ui/qml/SettingsPane.qml, crates/nidvue-ffi/src/lib.rs | replay: cargo xtask ui && cargo xtask ui-smoke
- author evidence: cargo xtask ffi-smoke @7d1c7ad -> the C consumer asserts config list carries `cache.max_size_bytes` and `sync.pairs`
- reviewer evidence: cargo xtask ui-smoke @7d1c7ad -> 48 rows, no QML errors; cargo xtask ffi-smoke -> config list/get/set/refusal/peek all pass
- findings history: the first smoke test only proved the window instantiated, which a broken data path would also pass; the reviewer required the row count to be asserted
- branch / merge: feat/settings-and-live-cache / 7d1c7ad

## T-0037 · The cache cap applies without a restart  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1 (recorded verdict on pull request 16)
- task / issue: -
- area: cache
- self-check: [pass] Re-read the change: `Cache` keeps the cap in an `AtomicU64`; `set_capacity` refuses zero, stores the value and evicts to the new low watermark before returning; `put`, `put_file`, `evict_to_capacity` and `effective_capacity` read the atomic, and `ensure_room` takes the cap as a parameter so a write obeys the live value rather than the one the cache opened with. The SDK applies it from `apply_live` on every config write, so the CLI, the Settings slider and the Advanced card reach the same path. Concurrent writes serialise on the atomic; a zero is refused and changes nothing. | test: cargo test -p nidvue-cache -p nidvue-sdk @7d1c7ad -> 23 + 6 passed, 0 failed | artifacts: crates/nidvue-cache/src/lib.rs, crates/nidvue-sdk/src/lib.rs | replay: cargo test -p nidvue-cache the_capacity_is_live
- author evidence: cargo test -p nidvue-cache the_capacity_is_live_and_evicts_when_lowered @7d1c7ad -> 1 passed (usage drops to <= the new cap, the next write obeys it)
- reviewer evidence: cargo test -p nidvue-sdk the_cache_cap_applies_without_a_restart @7d1c7ad -> 1 passed (config_set, set_cache_limit_bytes and a reopen all agree)
- findings history: -
- branch / merge: feat/settings-and-live-cache / 7d1c7ad

## T-0038 · `ui.start_minimised` and `ui.theme` are honoured  (developer)
- date: 2026-11-16
- priority: P1
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1 (recorded verdict on pull request 16)
- task / issue: -
- area: ui
- self-check: [pass] Re-read the change: `nidvue_config_peek` loads `config.toml` without opening the engine, and `main()` reads `ui.theme` and `ui.start_minimised` before `engine.load()`, so neither a theme flash nor a visible-then-hidden window is possible. A theme chosen in the window still wins over the configured default; `start_minimised` is ANDed with `QSystemTrayIcon::isSystemTrayAvailable()` so a desktop without a tray cannot start invisible. A missing file, an unknown key or a malformed value all fall back to the previous behaviour. | test: cargo xtask ffi-smoke @7d1c7ad -> peek reads false, then reads back true after a write | artifacts: crates/nidvue-ffi/src/lib.rs, apps/nidvue-ui/main.cpp, apps/nidvue-ui/preferences.cpp | replay: cargo xtask ffi-smoke
- author evidence: cargo xtask ffi-smoke @7d1c7ad -> "config_peek reads a bool" and "config_peek reads back what was written" pass
- reviewer evidence: grep -n 'isSystemTrayAvailable' apps/nidvue-ui/main.cpp @7d1c7ad -> the tray guard is on the same line as the config read; the tray-gated branch cannot be exercised offscreen, recorded as a limitation in the verdict
- findings history: -
- branch / merge: feat/settings-and-live-cache / 7d1c7ad

## T-0040 · A keyring that cannot answer no longer stops the client  (developer)
- date: 2026-11-16
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1 (recorded verdict on pull request 16)
- task / issue: -
- area: auth
- self-check: [pass] Re-read the change: `AuthManager::restore` treats a store read error as "no token yet" and logs the reason, so a platform keyring that answers the probe and then fails a read (observed as `DBus error: Client public key size is invalid`) degrades to "not logged in" instead of aborting `Client::open`. A corrupt payload is still an error — that is a real inconsistency worth surfacing. `logout` and `login` are unchanged. | test: cargo test -p nidvue-jotta @7d1c7ad -> 31 passed, 0 failed | artifacts: crates/nidvue-jotta/src/auth.rs | replay: cargo test -p nidvue-jotta an_unreadable_store_does_not_stop_the_client
- author evidence: cargo test -p nidvue-jotta an_unreadable_store_does_not_stop_the_client @7d1c7ad -> 1 passed
- reviewer evidence: the failure was intermittent (one of two full-suite runs); the new test pins the behaviour with a store that always errors, so it no longer depends on the environment's D-Bus session
- findings history: -
- branch / merge: feat/settings-and-live-cache / 7d1c7ad

## T-0002 · Protect main and make the Reviewer the only approver  (release-engineer)
- date: 2026-11-16
- priority: P1
- owner: release-engineer role (work by: ReleaseEngineer)
- reviewer: Reviewer-1
- task / issue: -
- area: branch-protection
- self-check: [pass] Re-read the diff: two files, no code. CODEOWNERS has one rule, '*' -> the reviewer identity, and no other pattern so nothing escapes it. Release.md's table matches the protection the API returned, including the three settings that were previously implicit (last-push approval, code-owner switch, enforce_admins). The gap is stated with its switch and the constraint that keeps the count at 0; no claim is made that GitHub enforces reviewer-only approval today. Boundary: an empty CODEOWNERS or a pattern typo would silently request no review - checked by reading the file. Duplicate: no other CODEOWNERS exists (git ls-files .github/CODEOWNERS -> 1). Second call: the doc section is idempotent prose; re-applying it changes nothing because it replaces the same block. Scope: docs + .github only, no code path. | test: gh api repos/Nidvue/Drive/branches/main/protection -> last_push=true, codeowners=false, reviews=0, force=false, deletions=false, linear=true; git show --stat e9c1180 -> 3 files; git ls-files .github/CODEOWNERS -> 1 path | preflight: PF-b39ae43e34f3 @e9c1180323a4 (9 checks, 0 failed) || [pass] Re-read the amended diff: two files, no code. CODEOWNERS has exactly one rule, '*' -> the reviewer identity, so no path escapes it and no other pattern can shadow it. Release.md's table matches the protection the API returns, including the three settings that were implicit before (approval of the last push, the code-owner switch, enforce_admins). The gap is stated with its switch and the reason the count stays 0; the doc claims no forge-level enforcement today. Boundary: an empty file or a typo would silently request no review - read the file, one rule. Duplicate: git ls-files .github/CODEOWNERS -> 1. Second call: the section replaces the same block, so re-applying it is a no-op. Scope: docs and .github only. Correction from the first self-check: the commit helper had also appended 53 LFS patterns to .gitattributes; that is a separate concern, so it was dropped from this commit and is tracked as its own task. | test: git show --stat @b383e73 -> 2 files, 51 insertions; gh api repos/Nidvue/Drive/branches/main/protection -> last_push=true, codeowners=false, reviews=0, force=false, deletions=false, linear=true, admins=true, checks=5 strict | preflight: PF-cea216532c94 @b383e732beb8 (9 checks, 0 failed)
- author evidence: commit e9c1180
- reviewer evidence: git show b383e73 -> 2 files (CODEOWNERS 1 rule, Release.md 41 lines); gh api repos/Nidvue/Drive/branches/main/protection -> checks=5 strict, admins=true, last_push=true, codeowners=false, reviews=0, force=false, deletions=false, linear=true, conversation resolution=true; the table in Release.md matches that object field by field; teamwork.py pr merge --by Developer-1 -> refused ('only the Reviewer may merge'); teamwork.py review --by ReleaseEngineer -> refused ('only the Reviewer role may review')
- findings history: 
- branch / merge: releaseengineer/t-0002-protect-main-and-make-the-reviewer-the-o / 76af248

## T-0001 · Drop macOS and Windows from every active surface (keyring features, mount adapters, deny targets, CI and release matrices, packaging)  (developer)
- date: 2026-11-17
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Boundary: the crate now compiles only for Linux, so every cfg gate that named another platform is gone rather than left false. The empty/duplicate/second-call case: mount() has one implementation, and the fallback Unsupported arm that answered for unknown platforms went with the platform branches - the crate no longer pretends to build elsewhere. Contract surface: nidvue_mount::{mount, MountHandle, FuseVfs, MountOptions} is unchanged, so the CLI, SDK and FFI need no edit; the deleted packaging scripts are recoverable from tag v0.2.1 and the user authorised the deletion. Concurrency and shared writes: none touched; the secret.rs change in this branch is a separate bug task with its own commit. Scope: code, CI, packaging and NOTICE only; docs/deviations, packaging.md, README, ARCHITECTURE, Release, CHANGELOG, Plan and Board are the CEO's T-0001 and are not in this diff. | test: cargo test --workspace --all-features @ccb99ee -> every suite ok, 0 failed; cargo clippy --workspace --all-targets --all-features -- -D warnings @ccb99ee -> clean; cargo xtask arch|spdx|secrets|lfs @ccb99ee -> all green; rg -n 'target_os = "macos"|target_os = "windows"|macfuse|winfsp' crates/ @ccb99ee -> no match | preflight: PF-b31f14c83382 @ccb99ee8ba13 (10 checks, 0 failed) || [pass] Delta after review finding F1: the *.wasm LFS pattern is gone, so the example plugins stay real bytes and a clone without git-lfs still runs the plugin tests. Boundary unchanged: one platform, one adapter, no dead cfg branch. Empty/duplicate/second-call: the fallback Unsupported arm is gone with the platform branches, so mount() has exactly one implementation. Contract surface unchanged: nidvue_mount::{mount, MountHandle, FuseVfs, MountOptions}. Scope: the F1 fix touches .gitattributes only; nothing else changed since the failed revision. Mechanical: the full gate was re-run at the fixed head, not just the touched file. | test: cargo xtask ci @ca361b9 -> xtask ci: green; cargo xtask lfs @ca361b9 -> 0 tracked path(s) through Git LFS, 62 pattern(s), no text routed in; git checkout -- plugins/ @ca361b9 -> clean, no pointer warning | preflight: PF-b211d53ebd38 @ca361b9e62a2 (10 checks, 0 failed)
- author evidence: 
- reviewer evidence: delta review at ca361b9: F1 fixed (.gitattributes has no *.wasm; git checkout -- plugins/ -> clean, no pointer warning; cargo xtask lfs -> 62 patterns, 0 tracked through LFS, no text routed in); requirement re-run: rg -n 'target_os = "macos"|target_os = "windows"|macfuse|winfsp|apple-native|windows-native' crates/ Cargo.toml deny.toml packaging/ @ca361b9 -> no match; ls packaging -> linux.sh; cargo xtask arch @ca361b9 -> crate contract holds (11 crates) | artifacts: .pi/Teamwork/Teams/Drive/Agents/CEO/Reports/xtask-ci.log:6966201bc9180829:128838 | credited: TS-0006 feature/platform by TeamLeader @ca361b9 | baseline: failed at 4e07e73 on F1; the fix is one line in .gitattributes and the gate was re-run | replay: nix-shell --run 'cargo xtask ci'
- findings history: detects: a tracked real-binary routed into LFS by a pattern added with the constitution, which breaks every fresh checkout of the plugin examples | F1 .gitattributes line 49 routes *.wasm into Git LFS, but the two example plugins are committed as real bytes on purpose (the file's own comment at line 7 says a pointer would make their tests fail on a clone without git-lfs) and the plugin tests load them from the checkout. Evidence: git checkout -- plugins/example-*/com_example_*.wasm -> 'Encountered 2 files that should have been pointers, but weren't', and git status shows both wasm files modified after every cargo xtask ci. cargo xtask lfs misses it because the gate excludes .wasm from the text-in-LFS check by name. Fix: drop the *.wasm pattern; the other 50 added patterns catch no tracked file (checked with fnmatch over git ls-files).
- branch / merge: developer/t-0001-drop-macos-and-windows-from-every-active / 82df36f

## T-0004 · The secret scan reads any 'sk-' as an OpenAI key, so it flags the path xtask-ci.log  (developer)
- date: 2026-11-17
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Boundary: the gate must catch real credentials and nothing else; a prefix alone is not a credential, so the scanner now reads the key-shaped body after the prefix. Empty/duplicate/second-call: a bare 'sk-' is not a hit, the path xtask-ci.log is not a hit, a real-shaped sk-/ghp_ key is; the pattern table keeps one row per credential type. Contract surface: internal to xtask; the command and its exit codes are unchanged, and every other pattern keeps its previous behaviour (body 0 = contains). Scope: one function, one table, one test. Not a weakening: the body length is 20 for sk-/ghp_/github_pat_ and 12 for AKIA, all shorter than a real key. | test: cargo run -p xtask -- secrets @3b61a1f -> no credential-shaped strings in tracked files; cargo test -p xtask @3b61a1f -> 1 passed; cargo clippy -p xtask --all-targets -- -D warnings @3b61a1f -> clean | preflight: PF-74046a687e35 @3b61a1f7227c (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: review at 3b61a1f: cargo run -p xtask -- secrets -> no credential-shaped strings in tracked files (was FAIL: docs/Team/Archive.md: OpenAI-style key); cargo test -p xtask -> 1 passed, and the test proves both directions (a real-shaped sk-/ghp_ key is a hit, the xtask-ci.log path and a bare 'sk-' are not); the other six patterns keep body 0, so their behaviour is unchanged; the body lengths (20/20/20/12) are shorter than any real key, so the gate is not weakened | artifacts: .pi/Teamwork/Teams/Drive/Agents/CEO/Reports/xtask-ci-final.log:b0c1ad9f46e2f8f3:132017 | baseline: the gate was red at 9ff06dd on a false positive | replay: cargo run -p xtask -- secrets && cargo test -p xtask
- findings history: 
- branch / merge: developer/t-0004-the-secret-scan-reads-any-sk-as-an-opena / a9fe4ba

## T-0007 · The DCO step fails on the engine's own record commits, so every branch inherits a red gate  (developer)
- date: 2026-11-17
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Boundary: the exemption is two conditions ANDed - an exact subject prefix and every touched path under docs/Team/ - so a commit that names itself chore(records) but changes code is not exempt (unit test). Empty/duplicate/second-call: an empty path list is not exempt; a merge commit is skipped by the existing --no-merges; a signed record commit is skipped by the trailer check before the exemption is consulted. Contract surface: xtask only; the dco command's exit codes are unchanged and it still prints every offending SHA for a real miss. Concurrency and shared writes: none. Scope: xtask, the Release.md rule and the decision record; no product code. | test: cargo test -p xtask @056f758 -> 2 passed; cargo run -p xtask -- dco @056f758 -> 10 generated record commit(s) exempted, every authored commit carries a Signed-off-by trailer; nix-shell --run 'cargo xtask ci' @056f758 -> xtask ci: green | preflight: PF-a66934e73856 @056f758b79c6 (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: review at 056f758: cargo test -p xtask -> 2 passed, and the test proves the exemption cannot be abused (a chore(records) subject with a code path is not exempt, an empty path list is not exempt); cargo run -p xtask -- dco -> 10 generated record commits exempted and every authored commit signed, printed on every run; nix-shell --run 'cargo xtask ci' -> xtask ci: green including ffi smoke and ui smoke; Release.md states the exemption and its two conditions, so a reader cannot mistake it for a blanket waiver | artifacts: .pi/Teamwork/Teams/Drive/Agents/CEO/Reports/gate-dco.log:3d3625d6863d313c:129823 | baseline: dco was red on 13 unsigned engine commits at 64e0b0b, blocking every branch's gate | replay: cargo run -p xtask -- dco && nix-shell --run 'cargo xtask ci'
- findings history: 
- branch / merge: developer/t-0007-the-dco-step-fails-on-the-engine-s-own-r / 926cbbb

## T-0005 · Read ui.language: ship a translation, install a translator, expose the setting  (developer)
- date: 2026-11-17
- priority: P0
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Boundary: the translator is installed before the QML engine loads, so no frame renders in the wrong language; a locale with no catalog falls back to English and says so on stderr. Empty/duplicate/second-call: an empty ui.language falls back to the system locale, system resolves through QLocale, and a missing catalog is not an error. Contract surface: no C ABI change; the language control writes ui.language through the existing config_set, and the CMake configure step now needs lrelease (added to CI, the release workflow and shell.nix; the CI edit is authorised by the CEO). Concurrency and shared writes: none; the smoke now runs under a throwaway XDG_CONFIG_HOME so it cannot read or write the developer's QSettings or autostart directory. Scope: one coherent slice - the catalog, the loader, the control and the smoke that proves all three - because the feature is unprovable in pieces: a catalog with no loader is dead weight, and a loader with no assertion is untested. It is 13 files but one contract. | test: nix-shell --run 'cargo xtask ci' @deda8c7 -> exit 0, xtask ci: green (preflight gate check); nix-shell --run 'cargo xtask ui-smoke' @deda8c7 -> engine initialised, 48 settings rows, no QML errors, with theme: mode=dark, i18n: language=de catalog=:/i18n/nidvue_de.qm and a German window title asserted; lrelease -> 187 translation(s), 187 finished, 0 unfinished | preflight: PF-dc847b39dd85 @deda8c7688cd (10 checks, 0 failed)
- author evidence: 
- reviewer evidence: review at deda8c7: the reviewer ran the app under a config.toml with theme=dark and language=de -> theme: mode=dark dark=1, i18n: language=de catalog=:/i18n/nidvue_de.qm, and a QML window title rendered in German (Nidvue Drive – Einstellungen), which proves the catalog reaches bindings and not only the translator object; cargo xtask ui-smoke -> green under a throwaway XDG_CONFIG_HOME; lrelease -> 187 finished, 0 unfinished; the SettingsPane language control writes ui.language through the existing config_set and the Advanced card lists the same key | artifacts: .pi/Teamwork/Teams/Drive/Agents/CEO/Reports/reviewer-i18n.log:1ed34ff530f55545:584 | credited: TS-0011 integration/window by Tester @deda8c7 | baseline: deviation 14: ui.language reserved, and ui.theme silently ignored | replay: XDG_CONFIG_HOME=<tmp> apps/nidvue-ui/build/nidvue-ui --smoke-windows
- findings history: 
- branch / merge: developer/t-0005-read-ui-language-ship-a-translation-inst / 8f78cb2

## T-0006 · Measure the UI frame budget headlessly and record it  (developer)
- date: 2026-11-17
- priority: P1
- owner: developer role (work by: Developer)
- reviewer: Reviewer-1
- task / issue: -
- area: -
- self-check: [pass] Boundary: the measurement forces a paint (grabWindow) instead of timing the platform's frame signals, because the offscreen platform fires those without rasterising - proved by scaling the surface 16x and seeing no change. Empty/duplicate/second-call: a render that produced no image fails the check (painted=1 is asserted), and a second run is cheap and independent. Contract surface: a new --measure-frames argument on the app and a new xtask step in the gate; no ABI, no config key. Concurrency and shared writes: the measurement blocks the GUI thread deliberately, and runs in its own throwaway state directory and XDG_CONFIG_HOME. Scope: the measurement, the gate step and the three docs that carried the open budget. | test: nix-shell --run 'cargo xtask ci' @9bfd576 -> exit 0, xtask ci: green including '+ ui frames: ... p99=0.295 max=0.553 ms (budget 16 ms)'; cargo clippy -p xtask --all-targets -- -D warnings @9bfd576 -> clean; QT_SCALE_FACTOR=4 -> p50 8.897 ms at 4160x2240 (the cost scales, so the number is a render) | preflight: PF-0ac28cced7ad @9bfd576233a6 (8 checks, 0 failed)
- author evidence: 
- reviewer evidence: review at 9bfd576: the reviewer ran cargo xtask ui-frames itself -> p50 0.036 / p99 0.330 / max 0.702 ms against 16 ms with painted=1, and re-ran the scaling check the author used to prove the measurement is real (0.033 ms at 1040x560, 7.544 ms at 4160x2240); the step is in cargo xtask ci and the gate log shows '+ ui frames: ... (budget 16 ms)'; the docs state the two limits (software rasteriser, cost of a frame rather than a display refresh) instead of implying a GPU measurement | artifacts: .pi/Teamwork/Teams/Drive/Agents/CEO/Reports/reviewer-frames.log:3b980609603da8a8:1065 | baseline: deviation 7 listed the UI frame budget as unmeasurable | replay: nix-shell --run 'cargo xtask ui-frames'
- findings history: 
- branch / merge: developer/t-0006-measure-the-ui-frame-budget-headlessly-a / pending
