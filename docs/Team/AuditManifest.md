# Audit report

- generated: 2026-11-15T02:19:40
- team: Drive
- ledger: 112 records, chain verified: True
- checkpoint: record #112 (the ledger.export event behind this manifest; later records are work that happened after it)
- head hash: `559095d248a75949e1a9f1675d9a6962`
- git: ed7c3a663a22 on main, dirty: True
- externally verifiable artifacts: 4
- local-only state files: 42

## Verification (external, no trust required)

```sh
teamwork.py ledger verify    # recompute sha256 chain over Ledger.jsonl
sha256sum -c <(jq -r '.artifacts[] | "\(.sha256)  \(.path)"' docs/Team/AuditManifest.json)
teamwork.py trace <T-id>     # follow any claim end to end
```

## Artifacts

| path | bytes | sha256 |
|---|---|---|
| docs/Team/Architecture.md | 5198 | `053760bc7ffd5431…` |
| docs/Team/Decisions.md | 3713 | `e36b76202db2b97e…` |
| docs/Team/Plan.md | 6387 | `eb3b1752ab424e16…` |
| docs/Team/Project.md | 8977 | `a71baa92fac35829…` |
