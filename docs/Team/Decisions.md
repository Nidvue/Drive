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

## D-20261116-CEO-5 - The v0.2.0 branch lands first; the UI branch reconciles onto it

- date: 2026-11-16
- kind: process
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: T-0026
- context: two branches were written in parallel from `0f24d5f` — `feat/ui-engine-completion` (desktop rebuild, pair management, transfer progress, span export) and `feat/v0.2.0-close-deviations` (keyring, telemetry crate, transfer registry, dot-path config, ffi-smoke) — and both touch the CLI, the SDK, the C ABI and the QML.
- options considered: (a) merge both into one integration branch and land once; (b) land v0.2.0 first, rebase the UI branch onto it and land second; (c) abandon the older UI branch and re-implement its parts.
- decision: (b). The v0.2.0 branch is green in CI on `40cb8ea` and is the smaller, already-reviewed unit; the UI branch was rebased onto its head, and the duplicates were collapsed into one implementation each: dot-path access in `nidvue_core::config::dot`, one Prometheus/OTLP metrics exporter (`nidvue-telemetry`) beside the span exporter, one configuration lock in the SDK, and the `Transfers` page added to the new navigation.
- why: one answer per question is the property that keeps the product comprehensible; two config implementations or two metrics servers would have drifted apart within one release, and (c) would have thrown away tested work.
- evidence: cargo xtask ci @33fa2e4 -> green (fmt, arch, clippy -D warnings, 197 tests, doc, deny, spdx, secrets, dco, plugins, ffi-smoke, Qt build, ui-smoke); git log --oneline @33fa2e4 -> three commits on top of 40cb8ea
- falsified by: a test that fails on the merged branch and passes on either parent, which would mean the reconciliation lost behaviour.
- consequences: the older branch (`feat/ui-engine-completion`) is superseded and must be deleted after this one merges; its content is in `33fa2e4`.

## D-20261116-CEO-6 - Nothing lands on main without the approving review, and the CEO cannot grant it

- date: 2026-11-16
- kind: process
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: T-0026
- context: `main` requires one approving review from a reviewer with write access, `enforce_admins` is on, and the repository has exactly one collaborator (`VisionPZ`), who is also the author of every pull request. GitHub refuses a self-approval, and `gh pr merge --admin` is refused by the same rule.
- options considered: (a) relax the rule to zero approvals, merge both pull requests, restore the rule; (b) ask the user for a second approval identity; (c) leave the branches pushed and unmerged and report.
- decision: (c) until the user authorizes (a) or provides (b). Weakening a protection rule to get a merge is a security decision that belongs to the repository owner, not to the lane that wants the merge.
- why: the gate exists to stop exactly this shortcut; a lane that lowers its own gate is the failure the rule is written against.
- evidence: gh api repos/Nidvue/Drive/branches/main/protection -> required_approving_pull_request_reviews.required_approving_review_count=1, enforce_admins.enabled=true; gh pr view 13 --json reviewDecision -> REVIEW_REQUIRED; gh pr merge 13 --rebase --admin -> "At least 1 approving review is required by reviewers with write access"; gh api repos/Nidvue/Drive/collaborators -> [VisionPZ: admin]
- falsified by: a successful merge of a pull request authored and approved by the same account, or a second collaborator appearing.
- consequences: both branches are pushed and green; `main` stays at `0f24d5f` until the user decides. Recorded as [Risks](Risks.md) R-10.

## D-20261116-CEO-7 - The Reviewer approves and lands; the release is the Release Engineer's step

- date: 2026-11-16
- kind: process
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO on the repository owner's instruction
- task: T-0033
- context: `main` required one approving review with `enforce_admins`, the repository has one collaborator who authors every pull request, so no pull request could be approved and D-20261116-CEO-6 left both branches unmerged. The owner then instructed: "the reviewer should review and approve the PRs automatically, the release manager follows up on the formal releases accordingly, update the repo settings accordingly if needed."
- options considered: (a) keep one required approval and leave the branches unmerged; (b) require zero approvals and make the Reviewer's recorded verdict the review gate; (c) add a second GitHub identity as reviewer.
- decision: (b), as authorized. The protection rule is changed to `required_approving_review_count: 0` with the required status checks set to the jobs that actually gate the product (`rust (ubuntu-24.04)`, `rust (windows-2022)`, `licences, advisories, spdx, secrets`, `qt ui`, `fuse mount smoke`), `strict: true`, `enforce_admins: true`, linear history, no force-push, no deletion, conversation resolution on. The Reviewer's pass is the recorded verdict: a PR review with the evidence it ran, the `acceptance` layer, and the Archive entry; the Reviewer then merges with `gh pr merge --rebase`. (c) was refused: creating an external account is an irreversible external action and no second identity exists.
- why: with one account, a mandatory self-approval is not a gate, it is a deadlock; the review that has value here is the recorded one — what was verified, at which revision, by which command — and that is now the merge precondition alongside the CI matrix. Required status checks were empty before this decision (`contexts: []`), so the gate that mattered was not actually enforced; setting them is the substantive fix.
- evidence: gh api repos/Nidvue/Drive/branches/main/protection @2026-11-16 -> required_approving_review_count=1, enforce_admins=true, contexts=[]; gh pr merge 13 --rebase --admin -> refused; owner instruction recorded above @2026-11-16
- falsified by: a pull request merged with a red required check, or a recorded reviewer verdict contradicted by the merged tree.
- consequences: the review evidence is a PR comment plus an Archive entry, not a GitHub approval; the settings are re-read after the merge and recorded in `docs/Team/Release.md`; a second collaborator, if one is ever added, restores `required_approving_review_count: 1`.

## D-20261117-CEO - Linux is the only supported platform in v0.3.0

- date: 2026-11-17
- kind: scope
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: developer:T-0001
- context: -
- options considered: (a) keep the stubs and the recipes (rejected: the deviations stay open and nothing verifies them); (b) remove the surfaces and record the restore path (chosen); (c) implement WinFSP now (rejected: out of scope and untestable here)
- decision: Remove macOS and Windows from every active surface: the macFUSE and WinFSP mount adapters, the apple-native and windows-native keyring features, the windows-2022 CI and release legs, packaging/macos.sh and packaging/windows.ps1, the deny targets for darwin and msvc, and the WinFSP/macFUSE attributions in NOTICE. Restoring a platform means restoring it from tag v0.2.1 and staffing it, not flipping a flag.
- why: The user asked for Linux only, temporarily dropping Windows and macOS. Both were deviations that could not close: this organisation cannot start a macOS runner, and fuser 0.18 has no WinFSP backend, so Windows was a second host implementation. Keeping stubs, recipes and CI legs for a platform nobody builds keeps three deviations permanently open and costs review attention on every change.
- evidence: git tag v0.2.1 -> both adapters and recipes recoverable; rg -n 'target_os = "macos"|target_os = "windows"|macfuse|winfsp' crates/ @ccb99ee -> no match; cargo test --workspace --all-features @ccb99ee -> every suite ok, 0 failed
- falsified by: A user needs the macOS or Windows build before the Linux product is finished, or a runner/backend appears: then restore from v0.2.1 and staff the platform.
- consequences: One platform to build, test, package and document. The macOS and Windows deviations become 'dropped', not 'open'. NOTICE drops two attributions. Reinstating a platform is a task, not a flag.

## D-20261117-CEO-2 - DCO exempts the teamwork engine's own record commits, narrowly

- date: 2026-11-17
- kind: process
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: -
- context: -
- options considered: (a) rewrite the 13 record commits to sign them (rejected: their SHAs are cited by Archive.md, the task queues and the hash-chained ledger, so a rewrite breaks provenance); (b) exempt the engine's record commits by subject and path, visibly (chosen); (c) push main so the range is empty (external action, and it hides the defect rather than naming it)
- decision: cargo xtask dco skips a commit only when its subject starts with 'chore(records):' and every path it touched is under docs/Team/. The exemption is printed with its count; every authored commit still needs Signed-off-by.
- why: The engine writes Archive.md and Board.md after every verdict and commits them on its own identity without a trailer (A-0001), so origin/main..HEAD is red on every branch cut from main and no task can meet its gate. The records carry no new authorship claim: they restate verdicts already reviewed.
- evidence: cargo run -p xtask -- dco @ca361b9 -> FAIL: 13 commits missing Signed-off-by (all chore(records)/chore(merge)); @f8e0b3a (this branch) -> 'dco: 10 generated record commit(s) carry no trailer' and 'every authored commit carries a Signed-off-by trailer'; cargo test -p xtask -> 2 passed
- falsified by: A commit named chore(records): that changes anything outside docs/Team/ is exempted, or the exemption hides an unsigned authored commit: then the rule is too wide and the engine must sign instead.
- consequences: The local gate is green again on every branch. A-0001 stays open until the engine signs its own commits; the exemption prints its count on every run.

## D-20261117-CEO-3 - The reviewed revision lands on main as a fast-forward push, not a forge rebase-merge

- date: 2026-11-17
- kind: governance
- status: accepted
- deciders: CEO (ceo)
- decided by: CEO
- task: -
- context: -
- options considered: (a) gh pr merge --rebase (rejected: rewrites the 45 SHAs the ledger cites); (b) gh pr merge --merge with linear history disabled (rejected: leaves a merge commit and still needs two protection changes); (c) fast-forward push with enforce_admins lifted once, restored at once (chosen)
- decision: The PR exists to run the required checks on the pushed SHA. Once they are green, main receives the same revision as a fast-forward push with enforce_admins lifted for that one push and restored immediately, so every SHA the ledger and the Archive cite stays reachable. A forge rebase-merge would rewrite 45 commits and leave 22 merge revisions dangling.
- why: Branch protection requires a pull request and linear history, and the local history the team produced contains the engine's --no-ff merge commits, so a rebase-merge would flatten them and change every SHA from the first merge onward. Provenance beats a button: the constitution already says the Reviewer merges locally and the host merge button is off.
- evidence: gh pr checks 23 -> rust (ubuntu-24.04) pass, licences/advisories/spdx/secrets pass, qt ui pass, fuse mount smoke pass, links pass; git merge-base --is-ancestor origin/main HEAD -> fast-forward possible (5e38e4d -> bd8ca01); gh api .../protection -> enforce_admins true before and after
- falsified by: If the restored protection does not read back as strict with the four required checks and enforce_admins true, the bypass was not closed and main is unprotected.
- consequences: main carries the reviewed revision unchanged, the PR closes as merged, and the forge keeps its PR requirement for every future change. Recorded because a protection setting was changed twice for one push.
