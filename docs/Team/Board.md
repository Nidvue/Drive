# BOARD - live status

Owner: Project Manager. Machine view: `teamwork.py board`. This file is the human
summary plus the blocker list.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## Snapshot

- Date: 2026-11-16 · Milestone: M8 (usable end to end) · Verified criteria: 24/24
- Revision of record: `33fa2e4` on `feat/ux-parity-v2`, rebased onto
  `feat/v0.2.0-close-deviations` (`40cb8ea`). Not on `main`: the branch needs the
  one approving review the protection rule requires, and this environment has a
  single GitHub account, which cannot approve its own pull request. The blocker
  and the options are recorded in [Risks](Risks.md) R-10.
- Both parallel branches are green locally on the same gate
  (`cargo xtask ci`), and the v0.2.0 branch is green in CI on `40cb8ea`.
- Open feature: F-0001, the whole product as one slice.
- Queue state: ten tasks delivered in M7 and seven in M8, all self-checked and
  archived. Nothing blocked except the approval above.
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
| developer | T-0001 | - | done | - | - |
| developer | T-0002 | - | done | - | - |
| developer | T-0003 | - | done | - | - |
| developer | T-0004 | - | done | - | - |
| developer | T-0005 | - | done | - | - |
| product-manager | T-0001 | - | done | - | - |
| release-engineer | T-0001 | - | done | - | - |
| tester | T-0001 | - | done | - | - |
| tester | T-0002 | - | done | - | - |

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
