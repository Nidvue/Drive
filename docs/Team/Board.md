# BOARD - live status

Owner: Project Manager. Machine view: `teamwork.py board`. This file is the human
summary plus the blocker list.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## Snapshot

- Date: 2026-11-15 · Milestone: M6 (team records and close) · Verified criteria: 19/19
- Revision of record: `35ba7a1` on `main`; `origin/main` in sync at the time of writing.
- Open feature: F-0001 (`ready`), the whole product as one slice.
- Queue state: every role queue is empty. The delivered work predates task
  records, so there is nothing in flight and nothing blocked; `Plan.md` records
  the milestones and `Archive.md` the accepted results.

## Task table

One row per task in the shared role queues; `teamwork.py board --write` renders
this table and the In-flight table below from `Roles/*/TaskQueue.md`, so both are
the queues' state and not an opinion about it.

| role | id | area | state | pr | merged revision |
|---|---|---|---|---|---|

No rows: the queues are empty (see the snapshot note). The delivered history is in
`git log`, the criteria in `Criteria.md`, and the accepted results in
[Archive](Archive.md).

## Feature teams (rendered)

Shared services serve every feature and are attached to none: Architect, CEO, CFO, COO, CTO, Project Manager, Release Engineer, Reviewer, Security Auditor.

| feature | state | owner | squad | paths |
|---|---|---|---|---|
| F-0001 | ready | TeamLeader | none (single lane) | crates/, apps/nidvue-ui, plugins/, benches/, tests/invariants, packaging/, xtask/ |

## In flight

| agent | task | prio | state | artifact |
|---|---|---|---|---|

No open lane. The team is at close; a lane is staffed again by the first task that
needs one.

## Blocked

| task | waiting on | owner | unblock action |
|---|---|---|---|

Nothing is blocked.

## Duplicate or conflicting work

| what | owners | resolution |
|---|---|---|
| none | - | - |
