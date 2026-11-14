# ADR 0002 — Content-addressed blobs with segmented LRU-K eviction

Status: accepted · 2026-11-14

## Context

A mount needs random reads of large files at low latency, a cache bounded by a
user-set size, and eviction that never removes a file a process has open. The
naive designs are a single directory of files (unbounded directory size) and a
flat LRU (sequential scans evict the working set).

## Decision

- **Blob store**: immutable, content-addressed by SHA-256, sharded
  `blobs/ab/cd/<hash>`. A version change is a new blob; nothing is rewritten in
  place.
- **Metadata index**: SQLite is the only authority on what a path currently is.
- **Eviction**: segmented LRU-K — a probation segment, a protected segment
  promoted on the second access inside a window, and an access-frequency
  histogram. Pins (`open`, `dirty`, `in-flight`, `offline`) are absolute.
  Eviction runs incrementally between I/O in small batches, triggered by the
  high watermark and stopped at the low watermark; the hard cap is enforced by
  refusing a write that would exceed it after eviction.

## Consequences

- A sequential scan cannot evict the working set, because one access only puts a
  blob in probation.
- Deduplication is free: two paths with identical content share one blob, and
  the Allocate API's MD5 de-duplication lines up with the same idea remotely.
- The index and the blobs can disagree after a crash; the scrubber resolves it
  in the index's favour only after verifying the blob's hash, and a corrupt blob
  is deleted and re-fetched, never served.
