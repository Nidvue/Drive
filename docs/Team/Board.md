# BOARD - live status

Owner: Project Manager. Machine view: `teamwork.py board`. This file is the human
summary plus the blocker list.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## Snapshot

- Date: 2026-11-17 · Milestone: M11 (v0.3.0, Linux only) · Verified criteria: 6/6
  for this slice (AC-1 … AC-6).
- Revision of record: `82df36f` on `main` — the merge of
  `developer/t-0001-drop-macos-and-windows-from-every-active`; the gate
  (`cargo xtask ci`) is green at `ca361b9`, the branch head that was merged.
- Not tagged and not pushed: the tag and the push are the CEO's and the Release
  Engineer's separate steps, and no CI run exists on this revision yet.
- macOS and Windows are **dropped, not deferred**: their adapters, packaging
  recipes, CI legs and keyring backends left the tree and are recoverable from
  tag `v0.2.1` ([Decisions](Decisions.md) D-20261117-CEO).
- Open feature: F-0001, the whole product as one slice.
- Queue state: five tasks in M11, all self-checked, reviewed with a recorded
  verdict and archived. Nothing in flight, nothing blocked. One review failed
  (developer T-0001, finding F1) and passed as a delta at the fixed head.
- `loop` reports STOP and the ledger chain is intact.
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
| ceo | T-0001 | - | done | P-0001 | 82df36f |
| developer | T-0001 | - | done | P-0001 | 0a79d1b |
| developer | T-0002 | - | done | P-0002 | 3709df1 |
| developer | T-0003 | - | done | P-0003 | 6489eba |
| tester | T-0001 | - | done | P-0001 | 82df36f |

Rows whose PR is not merged carry no merged revision; the revision is read from
the PR record, not from this table.

No rows are open: every M11 task is `done`, so the queues are empty again. The
delivered history is in `git log`, the criteria in `Criteria.md`, and the
accepted results in [Archive](Archive.md) — read them by id with `teamwork.py
archive --task <T-id>` rather than opening the file.

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
