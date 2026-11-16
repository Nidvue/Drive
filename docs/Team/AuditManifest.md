# Audit report

- generated: 2026-11-16T04:16:16
- team: Drive
- ledger: 187 records, chain verified: True
- checkpoint: record #187 (the ledger.export event behind this manifest; later records are work that happened after it)
- head hash: `180babfdbb564657546588ff4226b040`
- git: 12130652f820 on main, dirty: False
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
| docs/Team/Decisions.md | 6462 | `23a3253361ec9d9f…` |
| docs/Team/Plan.md | 8317 | `215728461a3279c8…` |
| docs/Team/Project.md | 9570 | `0a71ab31b17c185b…` |
