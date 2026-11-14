<!-- SPDX-License-Identifier: AGPL-3.0-only -->
<!-- Copyright (C) 2026 Nidvue -->

## What changed

## Why

## How it was verified

```
$ cargo xtask ci
<result>
```

## Deliberately not done

## Checklist

- [ ] Every commit is signed off (`git commit -s`) and `cargo xtask ci` is green
- [ ] New public items are documented; new config values have a default, a CLI flag and a UI control
- [ ] No `unwrap`/`expect` outside tests; no blocking call in an async task
- [ ] Licence: any new dependency passes `deny.toml` and is added to `docs/licensing.md`
- [ ] Docs updated (`ARCHITECTURE.md`, ADR, `docs/deviations.md` when a budget is missed)
