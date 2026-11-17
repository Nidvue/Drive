# RISKS - risk register

Owner: COO. Reviewed at every integration point. No entry without an owner and a
mitigation - otherwise escalate to the CEO instead of recording it.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

| id | risk | category | likelihood | impact | mitigation / trigger | owner | status |
|---|---|---|---|---|---|---|---|
| R-1 | Jottacloud changes the undocumented API and a release breaks | tech | med | high | the API layer is one crate (`nidvue-jotta`) with a fake server; a breaking change is one PR, not a rewrite. Trigger: a 4xx/5xx shape no test covers | Architect | open |
| R-2 | Windows and macOS mount backends ship as `Unsupported` | delivery | high | med | closed in `v0.3.0`: both platforms are out of scope and their adapters are gone, so nothing is advertised that does not work | Architect | closed |
| R-3 | Credentials rest in an encrypted file when the platform keyring is unavailable | security | low | high | `secret.backend = auto` probes the OS store and uses it when it answers; the fallback is the ChaCha20-Poly1305 file with a `0600` key, and `doctor` prints the platform's own reason. Closed for any host with a Secret Service, a Security framework or a Credential Manager; still open for a headless session, which is the case this build machine proves | Architect | mitigated |
| R-4 | A conflict storm on a large pair fills the disk with copies | delivery | low | high | the cache hard cap and `min_free_disk_bytes` are enforced before a copy is written; the Conflict Center lists every copy | Developer | open |
| R-5 | No macOS CI runner means the macOS recipe is untested | delivery | high | med | closed in `v0.3.0`: the macOS recipe and job are removed with the platform; reopening the platform reopens this risk | Release Engineer | closed |
| R-6 | A plugin escapes its declared capabilities | security | low | high | WASM under wasmtime, no ambient authority, fuel/memory/wall-clock limits, traps contained; capability scoping is unit-tested | Developer | open |
| R-7 | The team workspace lives outside git and can be lost | process | med | med | this happened once: the local `.pi/Teamwork` tree was deleted and had to be rebuilt. Now the committed record set (`docs/Team/`) carries the decisions, criteria and archive; the ledger is exported at every close | CEO | open |
| R-9 | The review lane shared a session with the author, so independence is procedural rather than structural | process | high | med | every verdict cites a command the reviewer ran at the revision under review, not the author's line; `coordination --strict` counts the credit ratio (C-REVIEW-NOCREDIT) and the ledger keeps both runs. Trigger: a second model or a launcher becomes available, then `TEAMWORK_MODEL_ALT` and a separate lane restore structural independence | CEO | open |
| R-8 | Release artefacts are unsigned | release | med | med | SHA256SUMS ship with the tarball; signing needs a key and a policy, recorded as a follow-up rather than half-done | Release Engineer | open |

## Process health

- Cycle time: the delivered work predates task records; from `v0.1.0` every slice
  is a task (`Plan.md` M6).
- Blocked hotspots: none.
- Duplicate work seen: none; the criteria ledger has one owner per criterion.
- Tooling gaps: no launcher for a live lane in this environment, so the protocol
  runs as one agent wearing the hats sequentially (`references/Adapters.md`,
  degraded mode). R-9 is the consequence, and the mitigation is that the
  reviewer's evidence is always its own run at the revision under review.
- Cycle time in M7: ten tasks, one self-check each, zero review failures. The
  first drafts of both benches were wrong (the transfer bench measured the cache
  after a full-object download; the initial-sync bench forgot the 100
  directories) and the author caught both before submitting — which is the
  self-check doing its job, not a review saving it.

## Sources

- [`../deviations.md`](../deviations.md) — every unmet requirement with its reason, in-repo.
- [`../performance.md`](../performance.md) — measured budgets and the ones still unmeasured, in-repo.

## Open finding: plugin build output is tracked

`plugins/example-hello/target/` and `plugins/example-filter/target/` are committed,
so every `cargo xtask ci` run (which rebuilds the example plugins) dirties the
working tree with eight modified files that are build artefacts. `.gitignore` now
covers `**/target/`, which stops new files appearing, but tracked files need
`git rm -r --cached` plus a rebuild of the shipped `.wasm` — a deletion of tracked
files, so it waits for the owner's authorization rather than being done quietly.

- evidence: `git status --short | grep plugins/` -> 8 modified files after
  `cargo xtask ci`
- owner: Release Engineer

## R-10 - `main` cannot be merged into by the only account that has write access

- severity: high (blocks every landing, not a defect in the product)
- likelihood: certain while the repository has one collaborator
- owner: CEO
- statement: the branch protection rule requires one approving review and enforces it on admins; the single collaborator is the author of every pull request, so no pull request can be approved and `main` cannot advance.
- evidence: gh api repos/Nidvue/Drive/branches/main/protection @2026-11-16 -> required_approving_review_count=1, enforce_admins.enabled=true; gh pr merge 13 --admin -> refused with the same rule.
- mitigation: the user adds a second reviewer identity, or authorizes a recorded temporary exception (set the count to 0, merge the reviewed pull requests, restore it), or the repository is worked on from a fork whose pull requests carry two accounts.
- upgrade path: the decision and the authorization are recorded in [Decisions](Decisions.md) D-20261116-CEO-6 before any change to the rule.

## R-11 - The pre-0.2.0 commit history carries the owner's work email

- severity: low (the owner's own address, in the owner's own repository)
- likelihood: certain — it is in published history
- owner: Release Engineer
- statement: commits before the 0.2.0 release were authored as `Ping Zhao <ping.zhao@nidvue.com>`; every commit from 0.2.0 on uses a role mailbox (`<role>@agents.nidvue.com`). A history scan reports 216 findings, all of them that one address in commit metadata; the tree itself carries no credentials.
- evidence: `python3 privacy_scan.py --history --root .` -> 216 findings, all `personal-email` in commit author/message of commits older than 0.2.0; `git log --format='%an <%ae>' -6 main` -> role mailboxes only.
- mitigation: nothing to do for the shipped product — the release artefacts contain no author metadata. Rewriting published history to change the address is a red line (force-push, history rewrite) and needs the owner's explicit authorization, so it is not done.
- upgrade path: if the owner wants the address gone, the rewrite is a single authorized `git filter-repo --mail-map` plus a force-push, then a re-tag; recorded as a decision first.

## R-12 - `privacy_scan.py` flags a canary string in an invariant test

- severity: none (false positive, recorded so the next run does not re-triage it)
- likelihood: certain
- owner: Tester
- statement: `tests/invariants/tests/invariants.rs` contains the literal `eyJhbGciOiJIUzI1NiJ9.super-secret-refresh-token`, which is a fake token the test writes and then asserts is absent from the state directory in plaintext. The scanner reads it as an environment secret; the repository's own gate (`cargo xtask secrets`) passes because it looks for credential shapes that could be real.
- evidence: `python3 privacy_scan.py --tree --root .` -> 38 findings, all in that one test file; `cargo xtask secrets` -> `secrets: no credential-shaped strings in tracked files`.
- mitigation: the canary is the point of the test; renaming it would weaken the invariant.
- upgrade path: an allow-list entry for that file if the scanner becomes part of the gate.
