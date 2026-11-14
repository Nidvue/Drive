# ADR 0001 — Journal every mutation before applying it

Status: accepted · 2026-11-14

## Context

A sync engine that writes first and records later cannot answer "did this
upload finish?" after `kill -9`. The two failure shapes that lose user data are
a half-written local file and an upload whose server-side state is unknown.

## Decision

Every mutation is appended to a crash-safe journal (SQLite, WAL mode) **before**
it is applied, and marked committed after the bytes are durable and verified:

```
intent → fsync(journal) → apply(temp → fsync → rename → fsync dir) → commit
```

Recovery treats a row without a `commit` as *unknown*, never as done:

- upload → ask the server for the object's size and MD5; resume from
  `resume_pos` when the Allocate API supplies one, otherwise re-send;
- download → resume from the temp file's verified prefix, or restart;
- local write → re-hash the destination and compare with the journal's
  expected hash.

## Consequences

- Cost: one fsync per mutation, plus one for the commit. On a 100k-file initial
  sync this is the dominant cost, which is why intents are batched per
  directory while remaining ordered per path.
- Benefit: no state is inferred. Restart is deterministic and testable with a
  real `SIGKILL` (see `tests/crash/`).
- Rejected: a pure "scan and compare" recovery, which cannot distinguish "not
  uploaded" from "uploaded but the listing is stale", and would re-upload or,
  worse, delete.
