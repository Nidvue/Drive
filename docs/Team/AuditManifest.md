# Audit report

- generated: 2026-11-17T14:15:05
- team: Drive
- ledger: 232 records, chain verified: True
- checkpoint: record #232 (the ledger.export event behind this manifest; later records are work that happened after it)
- head hash: `115c2aee712ae604f793aaf793e665d6`
- git: f541d49b5693 on main, dirty: False
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
| docs/Team/Decisions.md | 18388 | `993b11a720d63189…` |
| docs/Team/Plan.md | 15773 | `d062e4cf943b37b5…` |
| docs/Team/Project.md | 9582 | `5d569e7f9f0e5bb2…` |
