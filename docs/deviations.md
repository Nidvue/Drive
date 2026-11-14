# Deviations from the brief

Every requirement that is not fully met is listed here with the reason and the
upgrade path. An empty row is not allowed: "works" without evidence is not a
status.

Status at `v0.1.0`.

| # | Requirement | Status | Reason / upgrade path |
|---|---|---|---|
| 1 | macOS mount backend (macFUSE) | not compiled | The macFUSE headers are not available in this build environment, so `nidvue-mount` returns a typed `Unsupported` on macOS rather than shipping an unbuildable adapter. The VFS contract is platform-independent and covered by the Linux mount test; enabling macFUSE is a build-flag change plus a CI runner with the SDK. |
| 2 | Windows mount backend (WinFSP) | not implemented | `nidvue-mount` returns a typed `Unsupported` on Windows instead of pretending. The WinFSP host bindings are the remaining work; the VFS contract and errno mapping are platform-independent and already tested. |
| 3 | OS keyring for credentials | not wired | Tokens are encrypted at rest with ChaCha20-Poly1305 in `credentials.enc` (`0600` key file beside it). Threat model: protects against accidental disclosure (backups, log scraping, crash reports), not against an attacker who can read the state directory. A keyring backend slots in behind `SecretStore`. |
| 4 | OpenTelemetry export | config field only | `log.otel_endpoint` exists and is documented, but no OTLP exporter is wired. CI cannot assert an external collector; JSON/Prometheus-shaped status is the tested path. |
| 5 | Prometheus endpoint | config field only | `log.prometheus_bind` is documented; the endpoint is not served yet. `status --json` is the machine-readable path today. |
| 6 | 100k-file initial sync measurement | synthetic only | Measured against the fake Jotta server in `tests/`; the real service's behaviour is not a property this project can guarantee. The engine is bounded by the index, not by RAM (state lives in SQLite). |
| 7 | Cold read / sustained transfer / UI frame budgets | not measured | These need a real 50 Mbit link and a windowed session. `doctor --bench` measures the cache and metadata budgets, which are green; the rest are listed in `docs/performance.md` as pending rather than claimed. |
| 8 | Every config value has a CLI flag and a UI control | partial | All values are documented with defaults in `docs/configuration.md` and validated; `--state`, `--json`, `--token-stdin`, `--read-only` and `sync add` cover the common path. The long tail is TOML-only. |
| 9 | Qt end-to-end against a live account | manual | CI builds the UI, launches it headlessly (`QT_QPA_PLATFORM=offscreen`) and asserts it initialises the engine with no QML errors. A live account is not available to CI. |
| 10 | Virtualised 100k-row views | partial | Log and conflict lists use `ListView` with `reuseItems`; a 100k-row transfer list is not implemented because the transfer list itself is not exposed as a model yet (the dashboard shows aggregates). |
| 11 | OIDC login by username/password | not implemented | Only the Personal Login Token flow is supported, which is the flow Jottacloud documents for command-line clients. No password is ever collected or stored. |
| 12 | Native plugin dylibs | refused | `PluginHost::load` refuses a native plugin with a typed error. Loading one is a licence decision (it must be AGPL-3.0) and needs a trusted-dylib host; neither is half-done. |
