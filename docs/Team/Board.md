# BOARD - live status

Owner: Project Manager. Machine view: `teamwork.py board`. This file is the human
summary plus the blocker list.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## Snapshot

- Date: 2026-11-16 · Milestone: M10 (v0.2.1 released) · Verified criteria: 24/24
- Revision of record: `7d1c7ad` on `main`, tagged `v0.2.1`; pull request 16
  rebase-merged, CI green on `main` at the tagged revision, artefacts attached to
  the GitHub release.
- `v0.2.0` was `6b42079`; pull requests 13 and 14 merged there.
- Both branches are merged and deleted; `main` is the release revision.
- Both parallel branches are green locally on the same gate
  (`cargo xtask ci`), and the v0.2.0 branch is green in CI on `40cb8ea`.
- Open feature: F-0001, the whole product as one slice.
- Queue state: ten tasks in M7, eight in M8/M9 and one in M10, all self-checked,
  reviewed with a recorded verdict and archived. Nothing in flight, nothing
  blocked.
- Independence limitation, stated rather than implied: this environment has one
  agent and no launcher, so the review lane ran sequentially in the same
  session as the author's. Every verdict cites a command the reviewer ran at the
  revision under review, and the missing structural independence is recorded in
  [Risks](Risks.md) R-9.

## Task table

One row per task in the shared role queues; `teamwork.py board --write` renders
this table and the In-flight table below from `Roles/*/TaskQueue.md`, so both are
the queues' state and not an opinion about it.

| role | id | area | state | pr | merged revision |
|---|---|---|---|---|---|
| ceo | T-0001 | - | done | P-0001 | - |
| developer | T-0001 | - | done | P-0001 | 0a79d1b |
| developer | T-0002 | - | done | P-0002 | 3709df1 |
| developer | T-0003 | - | done | P-0003 | 6489eba |
| tester | T-0001 | - | done | P-0001 | - |

Rows whose PR is not merged carry no merged revision; the revision is read from
the PR record, not from this table.

No rows: every M7 task is `done`, so the queues are empty again. The delivered
history is in `git log`, the criteria in `Criteria.md`, and the accepted results
in [Archive](Archive.md) — read them by id with `teamwork.py archive --task
<T-id>` rather than opening the file.

## Feature teams (rendered)

Shared services serve every feature and are attached to none: Architect, CEO, CFO, COO, CTO, Project Manager, Release Engineer, Reviewer, Security Auditor.

| feature | state | owner | squad | paths |
|---|---|---|---|---|
| F-0001 | active | TeamLeader | - | crates/, apps/nidvue-ui, plugins/, benches/, tests/invariants, packaging/, xtask/ |

## In flight

| agent | task | prio | state | artifact |
|---|---|---|---|---|

The task queue (`teamwork.py task list <role>`) is authoritative for a task's
exact state; this table only says which lanes are open.

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
