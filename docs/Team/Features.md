# Features - feature teams (one record per vertical slice)

Owner: Project Manager.

> The independence map (docs/Team/Architecture.md 13). Every record's
> `paths` is its whole write set; `feature check` refuses two open records
> that own one path. Field authority: `owner`, `statement`, `paths`,
> `depends-on` and the `active`/`done`/`dropped` transitions belong to the
> record's owner; `interface` and the `ready` transition belong to the
> Architect; the id, the dates and every validation belong to the command.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)


## F-0001 · Nidvue Drive v0.1.0: two-way Jottacloud sync and a mountable filesystem, shipped as a Rust engine, SDK, CLI and Qt desktop app
- owner: TeamLeader
- interface: Architecture.md#3
- paths: crates/, apps/nidvue-ui, plugins/, benches/, tests/invariants, packaging/, xtask/
- depends-on: -
- state: ready
- created: 2026-11-15
- updated: 2026-11-15
