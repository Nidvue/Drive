# Audit report

- generated: 2026-11-17T11:23:45
- team: Drive
- ledger: 117 records, chain verified: True
- checkpoint: record #117 (the ledger.export event behind this manifest; later records are work that happened after it)
- head hash: `3c80d9f4b61191fbb6174fb9eefea24a`
- git: f1b65f58974e on main, dirty: False
- externally verifiable artifacts: 4
- local-only state files: 30

## Verification (external, no trust required)

```sh
teamwork.py ledger verify    # recompute sha256 chain over Ledger.jsonl
sha256sum -c <(jq -r '.artifacts[] | "\(.sha256)  \(.path)"' docs/Team/AuditManifest.json)
teamwork.py trace <T-id>     # follow any claim end to end
```

## Artifacts

| path | bytes | sha256 |
|---|---|---|
| docs/Team/Architecture.md | 5198 | `866bd7f937298ddc…` |
| docs/Team/Decisions.md | 14639 | `773c6e1974818e5d…` |
| docs/Team/Plan.md | 13301 | `24fa20f09dab998b…` |
| docs/Team/Project.md | 9582 | `5d569e7f9f0e5bb2…` |
