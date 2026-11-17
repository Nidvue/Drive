# EVOLUTION - the four books

Owner: CEO. The self-evolution record of this team: what it measures, what it owes,
what it changed and what it must promote. `teamwork.py evolution check` parses the
four tables below and fails on a row that cannot be recomputed, an overdue issue
that is still open, an unfalsifiable verdict or a promotion with no destination.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

Cut: the ledger record count at the time of writing. `teamwork.py throughput`
prints the same cut, so a number on this page is never free-floating.

## Metrics

Every row names the command that recomputes it. A row whose command is empty is a
number nobody can check, so the checker refuses it.

| metric | what it is | command | value at this cut |
|---|---|---|---|
| RFR | review fail rate: failed verdicts over all verdicts | `teamwork.py throughput` | 0 verdicts, so RFR is undefined, not 0% |
| Rework | deliverables that needed a second verdict | `teamwork.py throughput` | 0 of 0 targets |
| SFR | self-check fail rate: self-checks that failed over all self-checks | `teamwork.py throughput` | 0 self-checks recorded |
| TTV | time to verdict: claim to verdict, p50 and p90 | `teamwork.py coordination` | no claims yet |
| Verification waste | gates re-run that another role already recorded green at the revision | `teamwork.py coordination` | 0 duplicated gate runs |
| Record density | always-on bytes each instance loads | `teamwork.py harness budget` | 346715 B across 9 instances, cap 122880 B each |
| Freshness debt | known issues past their re-measure date | `teamwork.py evolution check` | 0 overdue |
| Promotion lag | lessons and knowledge not yet promoted to a skill or a rule | `teamwork.py lesson list` | 1 open lesson (L-0001) |
| Guardrails | red lines hit, gate overrides in force | `teamwork.py audit` | 0 red lines, 0 overrides |

## Known issues

Open means any status short of `closed`, `waived` or `escalated`. A date in the
future is the promise to re-measure; a date in the past is debt.

| id | issue | re-measure by | status |
|---|---|---|---|
| K-1 | macOS mount backend returns `Unsupported`; no macOS CI runner is available to this organisation | 2026-12-15 | superseded in `v0.3.0`: macOS is out of scope |
| K-2 | Windows mount backend returns `Unsupported`; WinFSP bindings are the remaining work | 2026-12-15 | superseded in `v0.3.0`: Windows is out of scope |
| K-3 | credentials rest in an encrypted file, not the OS keyring | 2026-12-15 | open |
| K-4 | cold read, sustained transfer and UI frame budgets are unmeasured; they need a real link and a windowed session | 2026-12-15 | open |
| K-5 | no OTLP exporter and no Prometheus endpoint; both are config fields only | 2026-12-15 | open |
| K-6 | the release artefacts are unsigned | 2026-12-15 | open |

## Evolution ledger

One row per change to how the team works. The verdict is one of `helped`,
`hurt`, `no observed effect` or `confounded`, and it may carry the measurement
that produced it in parentheses.

| id | change | verdict |
|---|---|---|
| E-1 | main is protected and every change lands through a rebase-only pull request with required checks | helped (two real merges, no direct push possible) |
| E-2 | CI runs the full matrix on every pull request instead of path-filtering documentation | helped (a docs-only pull request could never merge before) |
| E-3 | the crate contract became an executable check (`cargo xtask arch`) instead of a review note | confounded (added at close; no later change to measure against yet) |
| E-4 | the team record set moved into the committed tree (`docs/Team/`) instead of a local-only workspace | confounded (the local workspace was lost once before this change) |
| E-5 | the roster dropped to one Reviewer lane at close, with the second added back when a review queue exists | confounded (no review queue to measure throughput on) |

## Promotion backlog

A lesson or a fact that has no destination yet. Level `L1` is recorded only,
`L2` is a team skill, `L3` is a rule in `AGENTS.md`, `L4` is an automated gate.

| record | what it is | level now |
|---|---|---|
| L-0001 | a destructive `rm -rf` in a multi-command shell line deleted the local team workspace | L1 |
| K-0001 | CI gates on pull requests, not on a direct push to main | L2 |
| K-0002 | the Jottacloud API is undocumented and unsupported, so it stays in one crate | L3 |

## Sources

- `teamwork.py throughput`, `coordination`, `harness budget`, `evolution check`,
  `lesson list`, `audit` — the commands this page is recomputed from.
