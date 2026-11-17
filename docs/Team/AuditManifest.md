# Audit report

- generated: 2026-11-17T12:08:26
- team: Drive
- ledger: 170 records, chain verified: True
- checkpoint: record #170 (the ledger.export event behind this manifest; later records are work that happened after it)
- head hash: `e4a6309599cc2f962cc219bfef58fa7b`
- git: a9f263b8c901 on main, dirty: False
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
| docs/Team/Decisions.md | 16430 | `b8d1cfd2ca008083…` |
| docs/Team/Plan.md | 14312 | `183250b986eec5d7…` |
| docs/Team/Project.md | 9582 | `5d569e7f9f0e5bb2…` |
