# Changelog

All notable changes to this project are documented here.
The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-11-16

Closes the credential, telemetry and transfer-list deviations from `v0.1.0`.

### Added
- OS keyring credential store behind the existing `SecretStore` seam, selected
  by `secret.backend` (`auto` by default): Secret Service on Linux, the Security
  framework on macOS, the Credential Manager on Windows. `auto` probes the
  platform store and falls back to the encrypted file with the reason recorded;
  `doctor` prints which store is live and why. The Linux backend needs
  `libdbus-1-dev` at build time (already a runtime dependency of every Linux
  desktop) and is declared in the CI and packaging steps; an explicitly
  requested `keyring` that cannot work is a failed `doctor` check, never a
  silent downgrade.
- `nidvue-telemetry`: a Prometheus scrape endpoint (`log.prometheus_bind`, text
  exposition 0.0.4 on `/metrics` and `/healthz`) and an OTLP/HTTP JSON metrics
  exporter (`log.otel_endpoint`, `<endpoint>/v1/metrics`, interval
  `log.otel_interval_s`). No `opentelemetry-*` dependency: the payloads are pure
  functions in `nidvue-core::metrics`.
- A bounded transfer list: `Status.transfers`, `transfer_limit`,
  `nidvue_client_transfers_json` / `nidvue_client_clear_transfers` in the C ABI,
  `Client::transfers` and `Client::publish` in the SDK, and a virtualised
  `Transfers` page in the UI (`ListView` with `reuseItems`, 512-row ceiling,
  live rows never evicted).
- `nidvue-cli config get|set|list|path`: every configuration key by dot path,
  parsed against the type already there and validated before it is written.
- `nidvue-cli sync add --include/--exclude/--conflict-policy/--paused`,
  `sync remove`, `sync pause`, `sync resume` and `sync list --json`.
- Per-pair exclude globs in Settings, a filter box on the Activity page, and tray
  notifications for a new conflict and for the first error of a run.
- `Client::config_get`/`config_set` and `Client::set_sync_pair_filters` in the
  SDK, `nidvue_client_config_get`/`_set` and
  `nidvue_client_sync_pair_set_filters` in the C ABI.
- `cache.capacity` in the status view, so the dashboard's cache bar has a real
  denominator instead of a zero.
- Benches: `transfer` (cold read, sustained transfer, warm read against an
  emulated 50 Mbit/s link) and `initial_sync` (100 000 paths: scan, index, plan,
  peak RSS).
- A design-token system (`qml/Theme.qml`) with full light and dark palettes, a
  modern font stack with fallbacks, and compact metrics.
- Automatic theme switching that follows the desktop colour scheme (Qt
  `QStyleHints`, then `kdeglobals`, then the Qt palette), with a persisted
  manual override (Match desktop / Light / Dark) in Settings.
- Sync pair management in the UI: add, remove and pause pairs, persisted to
  `config.toml`; a per-pair "Sync" action, the last pass summary per pair and
  the pair's exclude globs edited in place.
- Sync pass reports surfaced in the UI (applied / conflicts / failed per pair)
  and in the status bar, plus a filter box on the Activity page.
- Cache limit control and an installed-plugin list in Settings; log copy and
  clear on the Activity page.
- Per-file transfer list on the Overview, fed by `transfer_queued` /
  `transfer_done` / `transfer_failed` events; `TransferQueued` now carries the
  byte size, and `SyncEngine::subscribe` exposes the event stream.
- Byte-level transfer progress: `Remote` carries an optional progress callback,
  downloads report per chunk, uploads stream through a counting body, and the
  engine throttles `transfer_progress` to one event per 256 KiB plus completion.
  The transfer rows show a bar and `done / total`.
- OTLP/HTTP JSON **spans** for each finished sync pass, each failed transfer and
  each error-health transition (`log.otel_endpoint`, `/v1/traces`), alongside
  the metrics exporter above; the two signals share the endpoint and post to
  their own path.
- Tray notifications for a new conflict and for the first error of a run.
- `nidvue-cli sync add --include/--exclude/--conflict-policy/--paused`,
  `sync remove`, `sync pause`, `sync resume`, `sync list --json` and
  `sync run <name>`.
- `JOTTA_LOGIN_TOKEN` is honoured by `nidvue-cli login`.

### Changed
- The crate contract allows two HTTP egresses: `nidvue-jotta` for the API and
  `nidvue-telemetry` for metrics. `cargo xtask arch` enforces both the list and
  the rule that only `core` renders a metric payload.
- `stats()` reports real queue depth and active transfers from the transfer
  registry instead of two zeroes.

### Fixed
- `Transfers.qml` was missing from the QML resource list, so the page could not
  be loaded.
- `doctor` aborted instead of reporting when the engine could not open; it now
  records the failure as a check and returns the report.
- The Conflict Center's "Keep local" and "Keep remote" buttons did nothing: the
  labels the UI and the CLI document (`keep_local`, `keep_remote`) were not
  accepted by the conflict-policy parser. Both spellings parse now, with a test.
- `nidvue-cli … | head` panicked with a broken pipe instead of ending the output.
- A sync pair added or paused at runtime left `Client::config()` stale; the
  configuration is now one lock, updated by every write.
- The desktop app aborted on any desktop with a tray (`QWidget` created from a
  `QGuiApplication`); it now uses `QApplication`.
- Qt's compiled-QML disk cache served stale UI after a rebuild; the cache is
  disabled for the app.
- Sync pairs are held behind a lock, so a pair added at runtime is used by the
  next pass instead of waiting for a restart; the configuration is one lock, so
  `Client::config()` never goes stale after a write.
- `nidvue-cli … | head` panicked with a broken pipe instead of ending the output.
- Bench-only crates moved to `[dev-dependencies]`, clearing an unused-dependency
  warning, and workspace manifests no longer emit redundant-`homepage` warnings.

## [0.1.0] - 2026-11-14

First public pre-release.

### Added
- Rust workspace with one-directional crate boundaries: core, jotta, cache,
  vfs, sync, mount, plugin, sdk, ffi, cli.
- Jottacloud authentication with a Personal Login Token: OIDC token exchange,
  encrypted token persistence, refresh-on-401 with single-flight.
- JFS client: `liststream` listings, stat, mkdir, delete, move, download;
  Allocate API uploads with resume and MD5 verification.
- Token-bucket rate limiting, AIMD concurrency, exponential backoff with
  jitter, request coalescing.
- Content-addressed blob cache with a SQLite metadata index and segmented
  LRU-K eviction that honours pins, dirty and open entries.
- Journal-first sync engine: crash-safe WAL, three-way reconciler, conflict
  copies, durable queue, adaptive remote polling.
- FUSE3 mount on Linux (WinFSP and macFUSE adapters compiled on their
  platforms), with read-through reads and write-back spooling.
- WASM plugin host (wasmtime) with manifest, capability scoping and resource
  limits; two example plugins.
- Stable SDK with typed events, JSON schema and C header; C ABI bridge; Qt 6 +
  QML desktop application; headless CLI.
- Cross-platform tray icon with live sync health, pause/resume, sync now,
  mount/unmount, a compact status popup and a detailed settings window; closing
  the main window hides it to the tray.
- CI matrix (Linux, macOS, Windows), release workflow, CodeQL, dependency
  allowlist, SPDX header check and secret scan.
