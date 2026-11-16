# Licensing

Nidvue Drive is **AGPL-3.0-only**. This page maps every dependency and platform
SDK to its licence and how it is used. `cargo deny check licenses` enforces the
allowlist in `deny.toml`; a dependency that is not listed here is a review
failure.

## The project

| Component | Licence | Notes |
|---|---|---|
| Nidvue Drive (all crates, the Qt app, the plugins, scripts) | AGPL-3.0-only | SPDX header in every file; `LICENSE` is the verbatim AGPL-3.0 text |
| Example plugins | AGPL-3.0-only | a plugin shipped in this repository is part of it |

## Platform SDKs and native libraries

| Component | Licence | Linking | Obligations |
|---|---|---|---|
| Qt 6 — Core, Gui, Qml, Quick, Quick Controls 2, Quick Layouts, Network, Svg | LGPL-3.0 | **dynamic** | notice; user can replace the Qt libraries (no static link, no relink prevention) |
| Qt Charts / Virtual Keyboard / Data Visualization / Graphs | GPL-3.0-only | **not used** | avoided so the UI stays LGPL-compatible |
| libfuse3 | LGPL-2.1-or-later | **dynamic** (Linux) | notice; library unmodified |
| libdbus-1 | AFL-2.1 or GPL-2.0-or-later | **dynamic** (Linux) | notice; library unmodified; reached only by the keyring backend |
| WinFSP | GPL-3.0 with FLOSS exception | separate install, dynamic | permitted for this FLOSS project; attributed in `NOTICE` and in the MSI |
| macFUSE SDK | BSD-3-Clause | dynamic | attribution in `NOTICE` |
| Linux kernel FUSE, WinFSP service, macOS system extension | GPL-2.0 / GPL-3.0 / APSL | kernel interfaces | not linked into this project |

## Rust dependencies

The allowlist (also in `deny.toml`): MIT, MIT-0, Apache-2.0, Apache-2.0 WITH
LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, MPL-2.0, Unicode-3.0,
Unicode-DFS-2016, CC0-1.0, Unlicense, AGPL-3.0, GPL-3.0, CDLA-Permissive-2.0,
NCSA, OpenSSL.

Denied: GPL-2.0-only, LGPL with static-linking problems, SSPL, BUSL, Elastic
License, Commons-Clause, and anything with a non-commercial or no-derivatives
term. GPLv3 is accepted because it is one-way compatible **into** AGPLv3.

Direct dependencies and why each is allowed:

| Crate | Licence | Used for |
|---|---|---|
| tokio | MIT | async runtime |
| reqwest (rustls) | MIT/Apache-2.0 | HTTP; rustls avoids OpenSSL |
| rustls, ring | ISC/Apache-2.0/MIT | TLS |
| quick-xml | MIT | JFS XML streaming codec |
| serde, serde_json | MIT/Apache-2.0 | wire and config types |
| rusqlite (bundled) | MIT | journal, metadata index, sync state |
| sha2, md-5 | MIT/Apache-2.0 | content identity, Allocate MD5 |
| chacha20poly1305 | MIT/Apache-2.0 | encrypted credential file |
| keyring | MIT/Apache-2.0 | OS credential store (Secret Service, Security framework, Credential Manager) |
| dbus-secret-service | MIT | the Linux keyring backend, over the system `libdbus-1` (a runtime dependency of every Linux desktop; `libdbus-1-dev` to build) |
| fuser | MIT | FUSE3 bindings |
| wasmtime | Apache-2.0 WITH LLVM-exception | WASM plugin sandbox |
| notify | CC0-1.0 | filesystem events |
| globset | MIT/Apache-2.0 | include/exclude matching |
| clap | MIT/Apache-2.0 | CLI |
| tracing, tracing-subscriber | MIT | structured logs |
| schemars | MIT | JSON schema generation |
| zeroize | MIT/Apache-2.0 | wiping key material |
| (none new) | — | the OTLP exporter reuses `reqwest`; no `opentelemetry-*` crate is linked |

The authoritative, version-pinned list is `Cargo.lock` plus
`cargo deny list`; this table is the human-readable summary.

## Plugin licensing rule

- A **WASM** plugin runs in the sandbox and communicates through a defined ABI,
  so it is a separate work: it may carry any licence its manifest declares
  (including proprietary). The manifest's `license` field is shown at install.
- A **native** plugin (dylib) is loaded into the process and is treated as a
  derivative work: it **must be AGPL-3.0**, and the UI says so at install time.

## AGPL §13 (network use)

A desktop client does not trigger §13. Any future server-side component — a sync
relay, a hosted index, a web UI — would have to offer its source to its users.
This project has none, and adding one is a licence decision recorded in
`ARCHITECTURE.md` §13 and a new ADR.

## Contributions

DCO sign-off (`git commit -s`), no CLA. Contributors keep their copyright and
grant the project the AGPL-3.0 licence.
