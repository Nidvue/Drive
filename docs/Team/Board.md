# BOARD - live status

Owner: Project Manager. Machine view: `teamwork.py board`. This file is the human
summary plus the blocker list.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## Snapshot

- Date: 2026-11-17 · Milestone: M11 (v0.3.0, Linux only) · Verified criteria: 6/6
  for this slice (AC-1 … AC-6).
- Revision of record: `355fed3` on `main`, the merge of pull request 23. The
  branch head `bd8ca01` is what CI verified: `rust (ubuntu-24.04)`,
  `licences, advisories, spdx, secrets`, `qt ui`, `fuse mount smoke` and `links`
  all green, plus the local gate (`cargo xtask ci`: 38 test binaries, ffi smoke,
  ui smoke, ui frames, dco with its record-commit exemption printed).
- Pushed: `main` carries the reviewed revision with every SHA the ledger cites
  still reachable (decision D-20261117-CEO-3).
- macOS and Windows are **dropped, not deferred**: their adapters, packaging
  recipes, CI legs and keyring backends left the tree and are recoverable from
  tag `v0.2.1` ([Decisions](Decisions.md) D-20261117-CEO).
- `ui.language` is read: a German catalog of all 187 strings ships, installed
  before the first frame, and the smoke proves a translated window title.
  `ui.theme` is honoured for the first time - it never was, because the peeked
  value arrived quoted.
- The UI frame budget is measured and gated (`cargo xtask ui-frames`).
- Open feature: F-0001, the whole product as one slice.
- Queue state: nine tasks in M11, all self-checked, reviewed with a recorded
  verdict and archived. Nothing in flight, nothing blocked. One review failed
  (developer T-0001, finding F1) and passed as a delta at the fixed head.
- `loop` reports STOP and the ledger chain is intact.
- Two engine defects stay open as blockers because they live in the tool, not in
  this repository: `A-0001` (the engine's own record commits carry no
  `Signed-off-by`, so `xtask dco` exempts them by subject and path, visibly) and
  `A-0002` (`teamwork.py clean --team-wide` deletes the artifact logs a test
  record cites).
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
| ceo | T-0002 | - | done | P-0008 | 3ae4ac2 |
| ceo | T-0003 | - | done | P-0009 | f4f576d |
| ceo | T-0004 | - | done | P-0010 | f03200f |
| developer | T-0001 | - | done | P-0001 | 0a79d1b |
| developer | T-0002 | - | done | P-0002 | 3709df1 |
| developer | T-0003 | - | done | P-0003 | 6489eba |
| developer | T-0004 | - | done | P-0004 | a9fe4ba |
| developer | T-0005 | - | done | P-0006 | 8f78cb2 |
| developer | T-0006 | - | done | P-0007 | 877145e |
| developer | T-0007 | - | done | P-0005 | 926cbbb |
| developer | T-0008 | - | dropped | - | bd8ca01 |
| developer | T-0009 | - | done | P-0011 | c376d05 |
| developer | T-0010 | - | review | P-0012 | - |
| developer | T-0011 | - | review | P-0013 | - |
| developer | T-0012 | - | review | P-0014 | - |
| tester | T-0001 | - | done | P-0001 | 82df36f |
| tester | T-0002 | - | review | P-0015 | - |

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
| developer | T-0010 · Tray best-effort on a desktop with no status-notifier host: report it, never hide | P1 | review | - |
| developer | T-0011 · Measure the 100k-path initial sync against the fake Jotta server, not an empty remote | P1 | review | - |
| developer | T-0012 · OIDC password login: a real token grant with no password stored | P0 | review | - |
| tester | T-0002 · Live-account end-to-end test, gated on a token the runner provides | P1 | review | - |

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
