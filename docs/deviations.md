# Deviations from the brief

Every requirement that is not fully met is listed here with the reason and the
upgrade path. This file is updated as part of the release, not afterwards.

Status at `v0.1.0`: see the table.

| # | Requirement | Status | Reason / upgrade path |
|---|---|---|---|
| 1 | macOS mount backend (macFUSE) | compiled, not exercised in CI | GitHub's macOS runners cannot load a system extension. The adapter compiles and is unit-tested against the VFS trait; a real-mount test is skipped gracefully. |
| 2 | Windows mount backend (WinFSP) | compiled, not exercised in CI | WinFSP requires a service install on the runner; the adapter compiles and the VFS contract tests run cross-platform. |
| 3 | OS keyring for credentials | not wired | Tokens are encrypted at rest with ChaCha20-Poly1305 in `credentials.enc` (`0600`). Threat model and upgrade path are documented in `crates/nidvue-core/src/secret.rs`. A keyring backend slots in behind `SecretStore`. |
| 4 | OpenTelemetry export | opt-in, no collector in CI | The exporter is wired behind `log.otel_endpoint`; CI cannot assert an external collector. The JSON/Prometheus paths are tested. |
| 5 | 100k-file initial sync measurement | synthetic, not on the live service | Measured against the fake Jotta server with a 50 Mbit shaper; the real service's behaviour is not a property this project can guarantee. |
| 6 | Qt app end-to-end against the live service | manual, documented | CI builds and launches the UI headlessly (`QT_QPA_PLATFORM=offscreen`) and asserts the bridge delivers typed state; a live account is not available to CI. |

Each row is either closed in a later release or stays here with a number. An
empty row is not allowed: "works" without evidence is not a status.
