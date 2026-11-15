# Audit report

- generated: 2026-11-15T02:07:17
- team: Drive
- ledger: 107 records, chain verified: True
- checkpoint: record #107 (the ledger.export event behind this manifest; later records are work that happened after it)
- head hash: `04a84bc8fb61a5e0c9ba754049631dc6`
- git: e64bf7e4f7ae on feat/arch-conformance-and-team-records, dirty: False
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
| docs/Team/Decisions.md | 2588 | `ec1c98127cab36ba…` |
| docs/Team/Plan.md | 6387 | `eb3b1752ab424e16…` |
| docs/Team/Project.md | 8977 | `a71baa92fac35829…` |
