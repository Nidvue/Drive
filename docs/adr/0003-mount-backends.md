# ADR 0003 — One async VFS trait, three thin kernel adapters

Status: accepted · 2026-11-14

## Context

FUSE3 (Linux), WinFSP (Windows) and macFUSE (macOS) have different callback
signatures, threading models and error conventions. Duplicating engine logic in
each is how mount bugs become platform-specific and unfixable.

## Decision

`nidvue-vfs::Vfs` is an async trait with the POSIX surface the client
implements. Each platform adapter is a translation layer only:

- it owns a `tokio` runtime handle and `block_on`s each kernel callback (kernel
  callbacks are synchronous; there is no other honest option);
- it maps `NidvueError` to `errno` in one function per platform;
- it holds no policy: no retries, no conflict logic, no caching decisions.

Reads are read-through and writes are write-back through the cache and spool.
Open handles pin their blobs, which is what makes eviction safe while a file is
open. Per-inode async locks give POSIX-ish consistency without a global lock.

## Consequences

- Semantics are documented once (`docs/vfs-semantics.md`) and tested against the
  trait with an in-memory implementation, so the test suite runs without a
  kernel.
- Real-mount tests run on Linux CI (FUSE is available on GitHub runners) and are
  skipped gracefully where it is not.
- Hard links are not representable (a Jottacloud object is one path) and
  advisory locks are mount-local; both are documented rather than faked.
