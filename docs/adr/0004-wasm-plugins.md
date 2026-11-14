# ADR 0004 — WebAssembly as the default plugin sandbox

Status: accepted · 2026-11-14

## Context

Plugins need to transform content, filter paths and add UI panels. Loading a
native dylib into the process gives a plugin the user's whole account and makes
a panic in plugin code able to kill the engine and the UI. It also creates a
licensing question: a dylib loaded into an AGPL-3.0 process is arguably a
derivative work.

## Decision

- Default: **WASM under wasmtime**, with a manifest that declares the
  capabilities requested (paths, hosts, hooks, UI panels), enforced at every
  host call. Fuel, memory and wall-clock limits per call; a trap is contained
  and disables the plugin.
- Optional: native dylib behind an explicit "trusted" flag, and **must be
  AGPL-3.0**, stated in the UI at install time.
- WASM modules may carry any licence their manifest declares: they are separate
  works communicating through a defined ABI.

## Consequences

- The engine cannot be crashed by a plugin; the failure is an event.
- Plugin authors pay a serialisation cost at the host boundary, which is
  acceptable because hooks operate on whole buffers or small metadata.
- The sandbox is only as good as the host functions: every host call checks the
  capability set before touching the filesystem or the network, and there is no
  ambient authority to fall back on.
