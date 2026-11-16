# Deviations from the brief

Related: [Project](Team/Project.md) · [Plan](Team/Plan.md) · [Architecture](Team/Architecture.md) · [Risks](Team/Risks.md) · [Release](Team/Release.md) · [Research](Team/Research.md) · [Costs](Team/Costs.md) · [Design](Team/Design.md) · [configuration](configuration.md) · [performance](performance.md) · [jotta-api](jotta-api.md) · [packaging](packaging.md)

Every requirement that is not fully met is listed here with the reason and the
upgrade path. An empty row is not allowed: "works" without evidence is not a
status.

Status at `v0.2.1`. Closed so far: 3, 4 and 5 in v0.2.0; 8, 10 and 13 in v0.2.1.
Item 14 is narrowed to `ui.language`. Everything still in **Open** carries the
measurement or the build prerequisite that keeps it open — four of them need a
platform or a session this build environment does not have, and two are
deliberate refusals.

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

## Open

| # | Requirement | Status | Reason / upgrade path |
|---|---|---|---|
| 1 | macOS mount backend (macFUSE) | not compiled | `fuser` 0.18 supports macOS (`macfuse-4-compat`), so the adapter is the same code as Linux behind a different `cfg`; what is missing is the macFUSE headers at build time. `nidvue-mount` returns a typed `Unsupported` on macOS rather than shipping an unbuildable module. Enabling it is a build flag plus a runner with the SDK. |
| 1b | macOS CI job | omitted | This organisation's Actions cannot start a macOS runner: the job is cancelled before it runs (three attempts, no runner assigned). The macOS packaging recipe and the adapter contract exist; a runner is all that is missing. |
| 2 | Windows mount backend (WinFSP) | not implemented | `fuser` 0.18 has no Windows backend (its feature list is `libfuse`, `libfuse2`, `libfuse3`, `macfuse-4-compat`, `macos-no-mount`, `async-trait`, `tokio`, `serde`, `serializable`, `experimental` — no WinFSP), so this is a second host implementation against the WinFSP API, not a build flag. `nidvue-mount` returns a typed `Unsupported` on Windows instead of pretending. The VFS contract and the errno mapping are platform-independent and tested. |
| 6 | 100k-file initial sync measurement | synthetic, measured | Measured, but against a local filesystem and an empty remote, not the real service: `cargo bench -p nidvue-benches --bench initial_sync` scans 100 000 paths in 664 ms, indexes them in 861 ms, and plans the first pass in 57 ms with a 54 MiB peak RSS (566 bytes/path). What this does not cover is 100k network transfers, which are bounded by the link, not by the engine. Numbers in [`performance.md`](performance.md). |
| 7 | Cold read / sustained transfer / UI frame budgets | measured, UI frame not | Cold read and sustained transfer are measured against an emulated 50 Mbit/s link: 189 ms for 1 MiB (budget 400 ms) and 86.8 % of link capacity sustained (budget 80 %). The UI frame budget is still unmeasured — it needs a windowed session on a real display, and `cargo xtask ui-smoke` only proves the QML engine starts headless. |
| 9 | Qt end-to-end against a live account | manual | CI builds the UI, launches it headlessly (`QT_QPA_PLATFORM=offscreen`) and asserts it initialises the engine with no QML errors. A live account is not available to CI. |
| 9b | Tray on desktops without a status-notifier host | best-effort | `QSystemTrayIcon` needs a tray host; on a bare Wayland session without one the icon is absent and the app behaves as if it had no tray (closing the window quits). The controller checks `isSystemTrayAvailable()` rather than showing a dead icon. |
| 11 | OIDC login by username/password | not implemented | Only the Personal Login Token flow is supported, which is the flow Jottacloud documents for command-line clients. No password is ever collected or stored. |
| 12 | Native plugin dylibs | refused | `PluginHost::load` refuses a native plugin with a typed error. Loading one is a licence decision (it must be AGPL-3.0) and needs a trusted-dylib host; neither is half-done. |
| 14 | `ui.theme`, `ui.language`, `ui.start_minimised` | partial: `ui.language` unread | `ui.start_minimised` and `ui.theme` are read before the first frame through `nidvue_config_peek` (the app decides whether to show its window and which theme to start in before the engine exists). A theme the user picks in the window still wins over the configured default, and `ui.start_minimised` is ignored on a desktop with no tray so the app cannot start invisible. `ui.language` stays reserved: no translation ships. |

## Sources

- [`performance.md`](performance.md) — the measured numbers quoted above, with the command that produced each.
- [`configuration.md`](configuration.md) — every key, including `secret.backend`, `log.otel_endpoint` and `log.prometheus_bind`.
