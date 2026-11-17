# Audit report

- generated: 2026-11-17T15:32:06
- team: Drive
- ledger: 242 records, chain verified: True
- checkpoint: record #242 (the ledger.export event behind this manifest; later records are work that happened after it)
- head hash: `80a250ef973dc7aa5aa15ea002d97c63`
- git: 25066e3dc806 on main, dirty: False
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
