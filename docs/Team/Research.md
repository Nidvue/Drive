# RESEARCH - market, competitors, state of the art

Owner: Product Manager. Every claim carries a source and an access date; anything
unverified is marked UNVERIFIED.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## 1. Problem and users

Jottacloud ships a desktop application for sync but no supported way to mount the
account as a local filesystem, and no public SDK. A user with a large archive
wants two things the official client does not offer together: a mount whose warm
reads never hit the network, and a documented API a tool can drive. The users are
Linux-first power users, developers who script their backups, and anyone whose
archive is bigger than their disk.

## 2. Alternatives compared

| solution | approach | strengths | weaknesses | licence / price | relevance |
|---|---|---|---|---|---|
| Jottacloud official client | native sync app | supported, simple | no mount, no SDK, no plugins | free with the service | the baseline we must not regress on |
| rclone | generic remote over a CLI | many backends, mature | no mount cache of its own, no conflict copies, no UI | MIT | the tool our users compare against |
| this project | one engine, sync + mount, SDK + plugins | warm reads from cache, both versions kept, documented API | young, Linux mount only | AGPL-3.0-only | the deliverable |

## 3. State of the art

- Dominant technique for inbound change detection on a provider without webhooks
  is adaptive polling with a local scan cursor. That is what this client does,
  and the UI says so instead of claiming a socket. Evidence:
  [`../jotta-api.md`](../jotta-api.md) §5.
- Content-addressed caches with an LRU-K policy and pins are the standard answer
  to "mount a remote larger than the disk"; the design is in ADR 0002.
- Resumable upload requires hashing before the body is sent when the provider
  wants the digest up front — the Allocate API does. Evidence:
  [`../jotta-api.md`](../jotta-api.md) §3.

## 4. What practitioners ship

- Mount adapters are per platform: FUSE3 on Linux, WinFSP on Windows, macFUSE on
  macOS. The contract is portable; the adapter is not (ADR 0003).
- Plugin hosts converge on WebAssembly with explicit capabilities and resource
  limits because it contains a trapped plugin (ADR 0004).
- Desktop clients that hide to a tray need a status-notifier host; on a bare
  Wayland session the icon is absent and the app must behave as if it had no
  tray. Evidence: [`../deviations.md`](../deviations.md) item 9b.

## 5. Gap analysis

| gap | evidence | opportunity | risk | recommendation |
|---|---|---|---|---|
| no mount in the official client | product README, the mount mode this project ships | the differentiator | kernel backends per OS | ship Linux first, type the rest `Unsupported` |
| no public SDK | [`../jotta-api.md`](../jotta-api.md) §5 | an SDK others can build on | an undocumented API can break | isolate it in one crate |
| no plugin surface | ADR 0004 | user extensions without forking | sandbox escape | WASM with capabilities, no ambient authority |

## 6. Recommendation

Build the engine once and expose it twice (sync and mount), with the API isolated
in one crate, the cache content-addressed, and conflicts always kept. That is
what shipped in `v0.1.0`.

## Sources

- [Jottacloud](https://jottacloud.com) — the service, accessed 2026-11-15.
- [`../jotta-api.md`](../jotta-api.md) — the API surface implemented here, in-repo.
- [`../adr`](../adr) — ADR 0001-0005, in-repo.
- The vendor's public API documentation: **UNVERIFIED** — none is published, which
  is why the API layer is isolated (see [Risks](Risks.md) R-1).
