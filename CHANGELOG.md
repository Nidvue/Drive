# Changelog

All notable changes to this project are documented here.
The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- CI matrix (Linux, macOS, Windows), release workflow, CodeQL, dependency
  allowlist, SPDX header check and secret scan.
