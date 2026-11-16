# DECISIONS

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

## D-20261115-CEO - Protected main, rebase-only pull requests

- date: 2026-11-15
- kind: governance
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: -
- context: -
- options considered: -
- decision: main requires a pull request, the five required checks, linear history and no force-push; only rebase merges are allowed; head branches are deleted; administrators are not exempt
- why: no unreviewed change lands on the default branch, history stays linear, and every commit keeps its DCO sign-off
- evidence: PR #9 and #10 merged with --rebase; direct push rejected: 'protected branch hook declined' @35ba7a1
- falsified by: -
- consequences: -

## D-20261115-CEO-2 - Protected main, rebase-only pull requests

- date: 2026-11-15
- kind: governance
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: -
- context: -
- options considered: -
- decision: main requires a pull request, the five required checks, linear history and no force-push; only rebase merges are allowed; head branches are deleted; administrators are not exempt
- why: no unreviewed change lands on the default branch, history stays linear, and every commit keeps its DCO sign-off
- evidence: PR 9 and PR 10 merged with --rebase; direct push rejected: protected branch hook declined @35ba7a1
- falsified by: -
- consequences: -

## D-20261115-CEO-3 - Team tree named after the project; one Reviewer lane at close

- date: 2026-11-15
- kind: process
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: -
- context: -
- options considered: -
- decision: the team tree is .pi/Teamwork/Teams/Drive, matching the project directory name, so the default command resolution and the tree agree; the roster runs one Reviewer lane while the review queue is empty and a second lane is added with 'instance add reviewer --count 2' when a queue exists
- why: the engine resolves the team from the project directory name, so a differently named tree made every command without --team act on an empty workspace; a second Reviewer lane with no queue cannot change what runs next
- evidence: instance list -> 9 roles; graph audit -> no C-GRAPH-CHAFF; feature check -> 1 open feature, 0 problems @35ba7a1
- falsified by: -
- consequences: -

## D-20261115-CEO-4 - Close-out landed through a protected pull request, not a direct merge

- date: 2026-11-15
- kind: process
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: -
- context: -
- options considered: -
- decision: the v0.1.0 close-out (executable crate contract, committed team record set, one test-double fix) landed as pull request 11, rebase-merged into main with the full CI matrix green on both operating systems; the branch was deleted after the merge
- why: main is protected and no force-push is allowed, so a local merge cannot be pushed; the repository's own gate is the only path that both preserves linear history and proves the change on the merged content
- evidence: gh pr checks 11 -> rust (ubuntu-24.04), rust (windows-2022), fuse mount smoke, qt ui, licences/spdx/secrets, links all pass; gh pr merge 11 --rebase --delete-branch -> main at ed7c3a6 @ed7c3a6
- falsified by: -
- consequences: the engine's task/review queue was not used for this change: the record of it is this decision, the commit messages and the ledger, and the team ran in the single-agent degraded mode

## D-20261116-CEO - M7 review independence is procedural, and recorded as such

- date: 2026-11-16
- kind: process
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: -
- context: -
- options considered: a) refuse to close and leave ten tasks in review; b) record the verdicts with their real provenance plus the limitation; c) re-run every gate a second time to simulate independence
- decision: Accept the M7 verdicts although the review lane shared a session and a model with the author: each cites a command the reviewer ran itself at the revision under review, and the missing structural independence is recorded as Risks R-9 and lesson L-0003 instead of being papered over
- why: an unverifiable review is worse than a documented one; (a) blocks a verified milestone on an unavailable resource, (c) re-derives results the ledger already holds, which is exactly what C-REVIEW-NOCREDIT measures
- evidence: teamwork.py coordination --strict @f8d34c9 -> C-REVIEW-NOCREDIT 0/9 passes cited a credit; teamwork.py lesson add L-0003; docs/Team/Risks.md R-9
- falsified by: a second model or launcher becomes available and the verdicts change when a structurally independent lane re-runs them
- consequences: the credit ratio is now tracked; the next milestone should cite peer records with --credit rather than re-running them

## D-20261116-CEO-2 - Two HTTP egresses: jotta for the API, telemetry for metrics

- date: 2026-11-16
- kind: technical
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: -
- context: -
- options considered: a) put the exporter in core, which would make the root crate an HTTP client; b) put it in jotta, which would make a Jottacloud client speak OTLP; c) a peer crate with its own egress
- decision: Allow exactly two crates to depend on reqwest: nidvue-jotta (the Jottacloud API) and the new nidvue-telemetry (Prometheus and OTLP). cargo xtask arch enforces the closed list and keeps the rule that only nidvue-core renders a metric payload. Recorded by the CEO because the architect instance was folded; the reasoning is the architect's and the check is executable
- why: core must stay dependency-free at the bottom of the graph and jotta must stay about Jottacloud; a transport concern deserves a transport crate, and a closed list keeps the invariant checkable
- evidence: cargo xtask arch @f8d34c9 -> + arch: crate contract holds (11 crates); crates/nidvue-telemetry/Cargo.toml depends on reqwest, nidvue-core does not
- falsified by: a third crate needs HTTP and cannot reuse either client; then the list is wrong and the decision is revisited
- consequences: ARCHITECTURE.md section 1 and its rules table changed; docs/licensing.md gains the keyring and telemetry dependency trees
