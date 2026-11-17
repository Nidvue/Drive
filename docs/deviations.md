# Deviations from the brief

Related: [Project](Team/Project.md) · [Plan](Team/Plan.md) · [Architecture](Team/Architecture.md) · [Risks](Team/Risks.md) · [Release](Team/Release.md) · [Research](Team/Research.md) · [Costs](Team/Costs.md) · [Design](Team/Design.md) · [configuration](configuration.md) · [performance](performance.md) · [jotta-api](jotta-api.md) · [packaging](packaging.md) · [plugin-guide](plugin-guide.md) · [licensing](licensing.md)

Every requirement that is not fully met is listed here with the reason and the
upgrade path. An empty row is not allowed: "works" without evidence is not a
status.

Status at `v0.3.0`. Closed so far: 3, 4 and 5 in v0.2.0; 8, 10 and 13 in v0.2.1;
14 in v0.3.0. Items 1, 1b and 2 are **dropped, not
closed**: Linux is the only supported platform, so the macOS and Windows mount
backends are no longer requirements. Everything still in **Open** carries the
measurement or the build prerequisite that keeps it open — two need a session
this build environment does not have, and two are deliberate refusals.

## Closed in v0.2.0

| # | Requirement | Was | Now |
|---|---|---|---|
| 3 | OS keyring for credentials | not wired | `secret.backend = auto` (default) probes the platform store and uses it when it answers, with the encrypted file as the fallback and the reason recorded. `auto` is verified by `secret::tests::auto_always_lands_on_a_working_store`; the real Secret Service round trip is `secret::tests::keyring_round_trips_through_the_os_store`, which is `#[ignore]`d because it needs a running provider on the session bus — this build machine has a bus and no provider, so the fallback branch is the one that ran. |
| 4 | OpenTelemetry export | config field only | `log.otel_endpoint` exports OTLP/HTTP JSON to `<endpoint>/v1/metrics` every `log.otel_interval_s` seconds, through the existing HTTP client. No `opentelemetry-*` dependency: the payload is built in `nidvue-core::metrics` and posted by `nidvue-telemetry`. Verified by `exporter_posts_otlp_json` against a mock collector. |
| 5 | Prometheus endpoint | config field only | `log.prometheus_bind` serves `/metrics` (text exposition 0.0.4) and `/healthz` from the live snapshot. Verified end to end by `the_scrape_endpoint_serves_live_values`, which binds the real socket and reads the real response. |

## Closed in v0.2.1

| # | Requirement | Was | Now |
|---|---|---|---|
| 8 | Every config value has a CLI flag and a UI control | CLI only, UI partial | Every key is readable and writable by dot path from the CLI, the SDK and the C ABI, and the desktop app's **Advanced** card lists every leaf with an editable value (48 rows, asserted by `cargo xtask ui-smoke`). |
| 10 | Virtualised 100k-row views | partial | The transfer list is a `ListView` with `reuseItems` over a bounded model (512 rows, live rows never evicted); the log list is virtualised with a filter box. |
| 13 | Cache cap changes while the client runs | applies on restart | `Cache` keeps the cap in an `AtomicU64`; `set_capacity` evicts to the new low watermark before returning, and the SDK applies it on every config write, so the Settings slider and `config set cache.max_size_bytes` take effect at once. Watermarks, eviction policy and prefetch still need a restart. |

## Dropped with the platform support (v0.3.0)

Linux is the only supported platform. These three were requirements of the
original brief and are now out of scope; nothing in the tree pretends otherwise.

| # | Requirement | What was there | How to bring it back |
|---|---|---|---|
| 1 | macOS mount backend (macFUSE) | The adapter compiled behind `cfg(target_os = "macos")` and returned a typed `Unsupported`; `fuser` 0.18 supports macOS through `macfuse-4-compat` | Restore `crates/nidvue-mount/src/lib.rs` from tag `v0.2.1`, add the `macfuse-4-compat` feature, and build on a runner with the macFUSE SDK. The `Vfs` contract is unchanged. |
| 1b | macOS CI job | Omitted: this organisation's Actions cannot start a macOS runner (the job is cancelled before it runs) | A macOS runner, then the job back in the `ci` and `release` matrices. |
| 2 | Windows mount backend (WinFSP) | A typed `Unsupported` stub; `fuser` 0.18 has no WinFSP backend, so this is a second host implementation, not a build flag | Restore the stub and the `windows-native` keyring feature from `v0.2.1`, then implement the WinFSP host against the same `Vfs` trait. |

## Open

| # | Requirement | Status | Reason / upgrade path |
|---|---|---|---|| 6 | 100k-file initial sync measurement | closed in v0.4.0 | `cargo bench -p nidvue-benches --bench initial_sync` now measures the remote side as real state: it builds the 13.8 MiB `liststream` the API returns for 100 000 entries, parses it with the production parser (93 ms), and plans against it (75 ms) after scanning 100 000 paths (375 ms) and indexing them in one transaction (76 ms); engine total 619 ms. What is still not measured is the HTTP transfer of those bytes or of the files — bounded by the link, not by this code — and the bench says so. The measurement also exposed that per-row indexing costs 2.9 ms/row (290 s for 100k); `Index::upsert_entries` batches it to 76 ms. Numbers in [`performance.md`](performance.md). |
| 7 | Cold read / sustained transfer / UI frame budgets | closed in v0.3.0 | Cold read and sustained transfer are measured against an emulated 50 Mbit/s link: 189 ms for 1 MiB (budget 400 ms) and 86.8 % of link capacity sustained (budget 80 %). The UI frame budget is measured headlessly by `cargo xtask ui-frames`, which is in the gate: p50 0.039 ms, p99 0.303 ms, max 0.682 ms for a 1040×560 frame against 16 ms. It uses Qt's software rasteriser offscreen, so it is an upper bound and not a GPU measurement on a real display — that part stays unmeasured and is not claimed. "No frame blocked by engine work" holds by construction: every engine call runs on the worker thread. |
| 9 | Qt end-to-end against a live account | partial: the engine round trip is automated, the window is not | CI builds the UI, launches it headlessly (`QT_QPA_PLATFORM=offscreen`) and asserts it initialises the engine with no QML errors. The account round trip is now a real test — `crates/nidvue-cli/tests/live.rs`, run with `JOTTA_LOGIN_TOKEN=<token> cargo test -p nidvue-cli --test live -- --ignored` — which logs in, pushes a file through a sync pair, pulls it into a second folder, compares the bytes and removes the pair; it refuses to pass without a token instead of skipping silently. It has **not been executed here**: this environment has no account, and calling the live service is an external action. What stays manual is the window against a live account, because that needs a display and a session. |
| 9b | Tray on desktops without a status-notifier host | closed in v0.4.0 | There is no way to draw a tray icon on a desktop with no status-notifier host, so the app says what that means instead of silently losing the feature: a banner in the main window and a line in Settings state that closing the window quits and stops syncing, the controller checks `isSystemTrayAvailable()` rather than showing a dead icon, and `ui.start_minimised` is ignored there so the app cannot start invisible. `cargo xtask ui-smoke` runs with no tray and fails if the banner is missing. |
| 11 | OIDC login by username/password | closed in v0.4.0 | `nidvue-cli login --username <user> --well-known <discovery-url> --token-stdin` runs the same OIDC password grant the token exchange uses (`client_id=jottacli`, `grant_type=password`, `scope=openid offline_access`), with the account password read from stdin into a `SecretString`, used for one request and dropped. The discovery URL is required because it is what a Personal Login Token would have carried; the flow has no default endpoint, and this repository does not claim one it has not verified. Verified by two tests against a mock server: the grant's form fields and the stored token, and a 401 whose error names the status without echoing the password. |
| 12 | Native plugin dylibs | closed in v0.4.0 | A trusted-dylib host exists: `PluginHost::with_native` loads a `plugin.so`/`plugin.dll` through `libloading` behind a versioned C ABI, and only when `plugin.allow_native = true` **and** the manifest declares AGPL-3.0-only. Both refusals name themselves, and the ABI version is checked before anything is called. Verified by `crates/nidvue-plugin/tests/native.rs`, which compiles a C fixture with `cc` and exercises load, hook call, untrusted refusal, licence refusal and a non-library file. |
| 15 | Replacing a file that already exists on the server | open, measured | `Allocate` answers 409 for a path that exists, so a second upload of the same path fails. Deleting the old version first was tried and **lost the file**: the pass reported success and the account no longer had it, so the client now reports the conflict instead of working around it. What works: a new path, a rename (server-side move), and a delete. The fix is a documented replace/overwrite call or a trash-then-allocate sequence that is verified before it is trusted. Evidence: `nidvue-cli sync run` -> `already exists` for an existing path, and the account listing after the delete-then-retry attempt. |
| 16 | Deleting a directory on the server | open, measured | `DELETE` and `POST ?rm=true` both answer success for a folder and the folder stays in the listing; file deletion works. Until this is resolved, a local directory removal propagates as an error, not as a silent success, and the empty folder is left on the account. |
| 17 | Two pairs on one remote folder | closed by a guard in v0.5.0 | Baselines are keyed by remote path, so a second pair that does not have a file locally deletes it remotely — measured on a live account, where one pair removed a file the other had just uploaded. `AppConfig::validate` now refuses two pairs whose remote folders are equal or nested, which is the only safe answer until baselines are keyed per pair. |
| 14 | `ui.theme`, `ui.language`, `ui.start_minimised` | closed in v0.3.0 | All three are read before the first frame through `nidvue_config_peek`. `ui.language` (`system`, `en`, `de`) installs the matching catalog from `:/i18n/nidvue_<lang>.qm` before the QML engine loads, and a German translation of all 187 strings ships; a language the build has no catalog for falls back to English with the reason on stderr. `ui.start_minimised` is ignored on a desktop with no tray so the app cannot start invisible, and a theme picked in the window still wins over the configured default. Verified by `cargo xtask ui-smoke`, which now runs under a throwaway `XDG_CONFIG_HOME` and fails when `theme: mode=dark`, `i18n: language=de catalog=...` or a translated window title is missing. |

## Sources

- [`performance.md`](performance.md) — the measured numbers quoted above, with the command that produced each.
- [`configuration.md`](configuration.md) — every key, including `secret.backend`, `log.otel_endpoint` and `log.prometheus_bind`.
