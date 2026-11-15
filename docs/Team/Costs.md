# COSTS - budget, unit economics, ROI

Owner: CFO. Every number states its assumption and check date. Escalate early if a
cap is projected to be hit.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

The CFO role is not staffed in this roster: the CEO records the numbers below
from measured artefacts. Add a CFO when the project acquires a real budget line.

## Cost model

| driver | unit | rate | volume | monthly | note |
|---|---|---|---|---|---|
| hosting | — | 0 | 0 | 0 | no service is hosted; the client is the product |
| CI minutes | GitHub Actions minutes | included for a public repository | every push and pull request, Linux + Windows matrix | 0 | assumed included; the assumption is in [Risks](Risks.md) R-5 |
| build machine | developer workstation | sunk | 1 | 0 | 49 GB of `target/` on the reference machine |
| team tokens | model calls per lane | provider rate | measured, not assumed | — | `teamwork.py harness budget` reports the always-on context; `throughput` reports the surface bytes |

## Budget

- Approved: zero recurring spend. There is no paid infrastructure, no vendor
  subscription and no hosted component.
- Hard caps and behaviour at cap: CI minutes are the only metered resource; the
  matrix drops a job before it drops a gate (`docs/deviations.md` item 1b removed
  the macOS job for availability, not cost).
- Current burn: 0.

## Unit economics

- Cost per user: 0. The client is downloaded and runs on the user's machine; the
  only external dependency is the user's own Jottacloud subscription.
- Cost per release: the CI run plus one developer session.
- Break-even assumption: not applicable — there is no revenue line to break even
  against. The project is AGPL-3.0-only and gives the client away.

## Build vs buy

| need | build | buy | recommendation | reason |
|---|---|---|---|---|
| Jottacloud sync engine | this workspace | the official client (no SDK, no mount) | build | the mount and the SDK are the reason the project exists |
| mount | `fuser` adapter | a commercial FUSE SDK | build | `fuser` is maintained and carries no licence cost |
| plugin sandbox | wasmtime | a hosted function service | build | no network dependency for a local plugin |
| crash reporting | none | a SaaS | neither | no telemetry is collected; see [`../../SECURITY.md`](../../SECURITY.md) |

## Sources

- [`../performance.md`](../performance.md) — the measured build and runtime numbers this page depends on, in-repo.
- [`../deviations.md`](../deviations.md) — what is not measured, stated as not measured, in-repo.
