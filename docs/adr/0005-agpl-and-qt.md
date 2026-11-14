# ADR 0005 — AGPL-3.0-only, dynamically linked Qt, no GPL-only modules

Status: accepted · 2026-11-14

## Context

The project must be AGPL-3.0-only, like its sibling `Nidvue/Teamwork`. Qt 6 is
the UI toolkit; some Qt modules are LGPL-3.0, some are GPL-3.0-only. libfuse3 is
LGPL-2.1; WinFSP is GPL-3.0 with a FLOSS exception; the macFUSE SDK is BSD-3.

## Decision

- `license = "AGPL-3.0-only"` in every manifest, SPDX header in every source
  file, `cargo deny check licenses` as a CI gate with an explicit allowlist.
- Qt is **dynamically linked** and limited to LGPL-3.0-compatible modules
  (Core, Gui, Qml, Quick, Quick Controls 2, Quick Layouts, Network, Svg).
  No Qt Charts, Virtual Keyboard, Data Visualization or Graphs: the graphs are
  QML `Canvas`. No static Qt, no stripping of the ability to relink.
- GPL-3.0 dependencies are allowed because GPLv3 is one-way compatible into
  AGPLv3 (§13). GPL-2.0-only, SSPL, BUSL, Elastic and non-commercial licences
  are denied.
- `NOTICE` records Qt, libfuse3, WinFSP and macFUSE and how each is linked;
  `docs/licensing.md` maps every dependency to its licence and use.
- No server-side component: AGPL §13 would then require offering source to
  network users. Adding one is a licence decision, recorded in
  `ARCHITECTURE.md`, not an incidental refactor.

## Consequences

- A fork that links Qt statically, or ships a GPL-2.0-only dependency, is not
  distributable as Nidvue Drive.
- The release workflow attaches the licence text and a written offer of
  corresponding source, and CI verifies both are present.
