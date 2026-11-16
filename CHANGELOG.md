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
- `cache.capacity` in the status view, so the dashboard's cache bar has a real
  denominator instead of a zero.
- Benches: `transfer` (cold read, sustained transfer, warm read against an
  emulated 50 Mbit/s link) and `initial_sync` (100 000 paths: scan, index, plan,
  peak RSS).

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
