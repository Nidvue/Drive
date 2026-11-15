# RISKS - risk register

Owner: COO. Reviewed at every integration point. No entry without an owner and a
mitigation - otherwise escalate to the CEO instead of recording it.

Related: [Project](Project.md) · [Plan](Plan.md) · [Board](Board.md) · [Architecture](Architecture.md) · [Decisions](Decisions.md) · [Research](Research.md) · [Design](Design.md) · [Risks](Risks.md) · [Costs](Costs.md) · [Release](Release.md) · [Archive](Archive.md) · [Features](Features.md) · [Evolution](Evolution.md)

| id | risk | category | likelihood | impact | mitigation / trigger | owner | status |
|---|---|---|---|---|---|---|---|
| R-1 | Jottacloud changes the undocumented API and a release breaks | tech | med | high | the API layer is one crate (`nidvue-jotta`) with a fake server; a breaking change is one PR, not a rewrite. Trigger: a 4xx/5xx shape no test covers | Architect | open |
| R-2 | Windows and macOS mount backends ship as `Unsupported` | delivery | high | med | typed error, documented in [`../deviations.md`](../deviations.md) items 1 and 2; the VFS contract is tested on Linux | Architect | open |
| R-3 | Credentials rest in an encrypted file, not the OS keyring | security | med | high | ChaCha20-Poly1305 with a `0600` key; threat model stated in `crates/nidvue-core/src/secret.rs`; a keyring backend slots in behind `SecretStore` | Architect | open |
| R-4 | A conflict storm on a large pair fills the disk with copies | delivery | low | high | the cache hard cap and `min_free_disk_bytes` are enforced before a copy is written; the Conflict Center lists every copy | Developer | open |
| R-5 | No macOS CI runner means the macOS recipe is untested | delivery | high | med | the recipe and adapter contract are in the tree; the gap is a runner, recorded as deviation 1b. Trigger: a macOS runner becomes available | Release Engineer | open |
| R-6 | A plugin escapes its declared capabilities | security | low | high | WASM under wasmtime, no ambient authority, fuel/memory/wall-clock limits, traps contained; capability scoping is unit-tested | Developer | open |
| R-7 | The team workspace lives outside git and can be lost | process | med | med | this happened once: the local `.pi/Teamwork` tree was deleted and had to be rebuilt. Now the committed record set (`docs/Team/`) carries the decisions, criteria and archive; the ledger is exported at every close | CEO | open |
| R-8 | Release artefacts are unsigned | release | med | med | SHA256SUMS ship with the tarball; signing needs a key and a policy, recorded as a follow-up rather than half-done | Release Engineer | open |

## Process health

- Cycle time: the delivered work predates task records; from `v0.1.0` every slice
  is a task (`Plan.md` M6).
- Blocked hotspots: none.
- Duplicate work seen: none; the criteria ledger has one owner per criterion.
- Tooling gaps: no launcher for a live lane in this environment, so the protocol
  runs as one agent wearing the hats sequentially (`references/Adapters.md`,
  degraded mode).

## Sources

- [`../deviations.md`](../deviations.md) — every unmet requirement with its reason, in-repo.
- [`../performance.md`](../performance.md) — measured budgets and the ones still unmeasured, in-repo.
