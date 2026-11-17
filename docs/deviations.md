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
|---|---|---|---|| 6 | 100k-file initial sync measurement | synthetic, measured | Measured, but against a local filesystem and an empty remote, not the real service: `cargo bench -p nidvue-benches --bench initial_sync` scans 100 000 paths in 664 ms, indexes them in 861 ms, and plans the first pass in 57 ms with a 54 MiB peak RSS (566 bytes/path). What this does not cover is 100k network transfers, which are bounded by the link, not by the engine. Numbers in [`performance.md`](performance.md). |
| 7 | Cold read / sustained transfer / UI frame budgets | closed in v0.3.0 | Cold read and sustained transfer are measured against an emulated 50 Mbit/s link: 189 ms for 1 MiB (budget 400 ms) and 86.8 % of link capacity sustained (budget 80 %). The UI frame budget is measured headlessly by `cargo xtask ui-frames`, which is in the gate: p50 0.039 ms, p99 0.303 ms, max 0.682 ms for a 1040×560 frame against 16 ms. It uses Qt's software rasteriser offscreen, so it is an upper bound and not a GPU measurement on a real display — that part stays unmeasured and is not claimed. "No frame blocked by engine work" holds by construction: every engine call runs on the worker thread. |
| 9 | Qt end-to-end against a live account | manual | CI builds the UI, launches it headlessly (`QT_QPA_PLATFORM=offscreen`) and asserts it initialises the engine with no QML errors. A live account is not available to CI. |
| 9b | Tray on desktops without a status-notifier host | best-effort | `QSystemTrayIcon` needs a tray host; on a bare Wayland session without one the icon is absent and the app behaves as if it had no tray (closing the window quits). The controller checks `isSystemTrayAvailable()` rather than showing a dead icon. |
| 11 | OIDC login by username/password | not implemented | Only the Personal Login Token flow is supported, which is the flow Jottacloud documents for command-line clients. No password is ever collected or stored. |
| 12 | Native plugin dylibs | closed in v0.4.0 | A trusted-dylib host exists: `PluginHost::with_native` loads a `plugin.so`/`plugin.dll` through `libloading` behind a versioned C ABI, and only when `plugin.allow_native = true` **and** the manifest declares AGPL-3.0-only. Both refusals name themselves, and the ABI version is checked before anything is called. Verified by `crates/nidvue-plugin/tests/native.rs`, which compiles a C fixture with `cc` and exercises load, hook call, untrusted refusal, licence refusal and a non-library file. |
| 14 | `ui.theme`, `ui.language`, `ui.start_minimised` | closed in v0.3.0 | All three are read before the first frame through `nidvue_config_peek`. `ui.language` (`system`, `en`, `de`) installs the matching catalog from `:/i18n/nidvue_<lang>.qm` before the QML engine loads, and a German translation of all 187 strings ships; a language the build has no catalog for falls back to English with the reason on stderr. `ui.start_minimised` is ignored on a desktop with no tray so the app cannot start invisible, and a theme picked in the window still wins over the configured default. Verified by `cargo xtask ui-smoke`, which now runs under a throwaway `XDG_CONFIG_HOME` and fails when `theme: mode=dark`, `i18n: language=de catalog=...` or a translated window title is missing. |

## Sources

- [`performance.md`](performance.md) — the measured numbers quoted above, with the command that produced each.
- [`configuration.md`](configuration.md) — every key, including `secret.backend`, `log.otel_endpoint` and `log.prometheus_bind`.
