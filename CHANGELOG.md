# Changelog

All notable changes to this project are documented here.
The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-11-17

Continuous sync, verified against a live account. Most of this release is what the
first real login exposed.

### Added
- **Continuous sync.** The engine watches every unpaused pair's folder, runs a pass
  on a local change and otherwise polls at an adaptive interval that doubles on an
  empty pass and is bounded by `sync.full_scan_interval_ms`. `nidvue-cli sync
  watch` runs it in the foreground; the app starts it with the window. Before this,
  `poll_interval_ms`, `max_poll_interval_ms`, `full_scan_interval_ms`,
  `debounce_ms` and `rename_window_ms` were configuration nobody read and sync only
  happened on an explicit `sync run`.
- **Renames are server-side moves.** A delete plus a create of the same content
  within `sync.rename_window_ms` becomes one `Move`; the remote is renamed and the
  local file moved, so a 2 GiB rename is not a 2 GiB upload.
- **A bulk-delete guard.** A pass that would delete more paths than
  `sync.bulk_delete_threshold` is refused and names the setting.
- **`nidvue-cli remote ls|rm`.** Ask the server what a folder contains, and delete
  one path (to the Jottacloud trash unless `--permanent`).
- **`Index::upsert_entries`**: one transaction for a whole scan.

### Fixed
- **Uploads never worked against the real API.** `allocate_path` produced
  `/jfs/JottaArchive/Archive/...` — the separator was missing and the mountpoint
  appeared twice — so every upload answered 409. The JFS URL had the same doubling
  (`/Archive/Archive/...` folders appeared in the account).
- A pair's remote folder is created on the first pass instead of failing the pass
  with a 404 from the listing.
- A listing of a missing path reports the path, not the XML body it came with.
- `DELETE` is used for a permanent delete; the file flag answered 404 for a folder.
- A pass no longer deletes the pair's own root, and a folder that already exists is
  not an upload failure.

## [0.4.0] - 2026-11-17

The five items this project had listed as refusals, or as "cannot be measured
here", are finished. Two of them turned into product fixes of their own.

### Added
- **Native plugins behind a trusted host.** `PluginHost::with_native` loads a
  `plugin.so`/`plugin.dll` through `libloading` over a versioned C ABI, only when
  `plugin.allow_native = true` and the manifest declares AGPL-3.0-only. The ABI
  version is checked before the first call, and every refusal names its reason.
- **OIDC login by username and password.** `nidvue-cli login --username …
  --well-known <discovery-url> --token-stdin` runs the same OIDC password grant as
  the token exchange. The password is read from stdin into a `SecretString`, used
  for one request and dropped: never stored, never in the config, never in a log
  line, and a rejected login reports the status without echoing it.
- **A desktop with no system tray says so.** A banner in the main window and a
  line in Settings explain that closing the window quits and stops syncing, so a
  bare Wayland session loses the tray feature without losing the information.
  `ui-smoke` asserts the banner on a tray-less desktop.
- **A live-account end-to-end test.** `crates/nidvue-cli/tests/live.rs` (ignored
  by default) logs in, pushes a file through a sync pair, pulls it back into a
  second folder, compares the bytes and removes the pair; it needs
  `JOTTA_LOGIN_TOKEN` and refuses to pass without one. It has not been executed in
  this environment — there is no account here.
- **`Index::upsert_entries`**: one transaction for a whole scan instead of one per
  row. Indexing 100 000 rows went from 290 s (2.9 ms/row, one fsync each) to 76 ms,
  which is what makes a first sync of a large tree practical.
- **The 100k-path benchmark plans against a real remote listing**, not an empty
  map: it builds the 13.8 MiB `liststream` the API returns for 100 000 entries and
  parses it with the production parser (93 ms) before planning (75 ms).

## [0.3.0] - 2026-11-17

**Linux only.** macOS and Windows support is dropped for now: their mount
adapters, packaging recipes, CI legs and keyring backends are removed from the
tree instead of shipping as stubs, and are recoverable from tag `v0.2.1`.

### Added
- **Start at login.** `nidvue-core::autostart` writes an XDG autostart entry
  (`$XDG_CONFIG_HOME/autostart/nidvue-drive.desktop`), `nidvue-cli autostart
  status|enable|disable` drives it headlessly, the C ABI exposes
  `nidvue_autostart_enabled` / `nidvue_autostart_set`, and the Settings window
  has a **Start at login** toggle that writes the same file. `cargo xtask
  ui-smoke` fails when the toggle is not built.

### Changed
- `cargo xtask ci` and the release workflow run one platform (`ubuntu-24.04`).
- `nidvue-cli doctor` checks `/dev/fuse` unconditionally; the Windows-only
  WinFSP check is gone with the platform.
- `deny.toml` judges the dependency graph for `x86_64-unknown-linux-gnu` and
  `aarch64-unknown-linux-gnu` only.
- `packaging/linux.sh` is the only recipe; `cargo xtask package` runs it.

### Removed
- `crates/nidvue-mount`: the macFUSE and WinFSP adapters and the `Unsupported`
  fallback for other platforms. The crate is FUSE3 on Linux, one adapter.
- `packaging/macos.sh`, `packaging/windows.ps1`, the `windows-2022` CI and
  release legs, the `apple-native` and `windows-native` keyring features, and
  the WinFSP/macFUSE attributions in `NOTICE`.

### Added
- **German translation and a working `ui.language`.** The window installs the
  catalog for `ui.language` (`system`, `en`, `de`) before the QML engine loads,
  so it is translated from the first frame; all 187 strings ship translated.
  The Settings window has a language control next to the theme choice.
- `cargo xtask ui-smoke` now runs under a throwaway `XDG_CONFIG_HOME` and asserts
  three first-frame settings: `theme: mode=dark`, `i18n: language=de
  catalog=:/i18n/nidvue_de.qm`, and a translated window title. It previously read
  the developer's own `QSettings`, which is why a configured theme could not be
  observed.

### Added
- `cargo xtask ui-frames` measures the cost of one rendered frame and is part of
  the gate: 1040×560 renders in p50 0.039 ms / p99 0.303 ms / max 0.682 ms against
  the 16 ms budget (software rasteriser, offscreen). The budget was previously
  listed as unmeasurable without a windowed session.

### Fixed
- `ui.theme` was never honoured: `nidvue_config_peek` answers with the JSON form
  of a value, so the app compared `"dark"` against `dark` and always started in
  `system`. The value is decoded now, and `ui-smoke` fails when the configured
  theme does not reach the first frame.
- The credential probe no longer races itself: two probes in one process share
  the Secret Service session, deleted each other's throwaway entry and reported
  a working store as broken. The probe is serialised and its key is per call, so
  the suite is deterministic instead of failing about once in twelve runs.

## [0.2.1] - 2026-11-16

### Added
- An **Advanced** card in Settings that lists every configuration key with an
  editable value, so the whole of `config.toml` is reachable from the window and
  not only the common settings. Backed by `nidvue_client_config_list_json`.
- `nidvue_config_peek`: read one value straight from `config.toml` without
  opening the engine, so the app can honour `ui.theme` and `ui.start_minimised`
  before the first frame.

### Changed
- The cache cap is **live**: `Cache` keeps it in an `AtomicU64`, `set_capacity`
  evicts to the new low watermark before returning, and the SDK applies it on
  `config set cache.max_size_bytes` (and from the Settings slider). Lowering the
  cap reclaims the excess immediately instead of at the next start.
- `ui.start_minimised` starts the app in the tray (ignored when the desktop has
  no tray, so it cannot start invisible); `ui.theme` seeds the starting theme
  and a choice made in the window still wins.

### Fixed
- `cargo xtask ui-smoke` now fails when the settings editor receives no rows, so
  a broken config path cannot pass the gate.

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
