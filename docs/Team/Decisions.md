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
