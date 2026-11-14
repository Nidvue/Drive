# Mount semantics — exactly what Nidvue Drive guarantees

The mount is a POSIX-ish view of one Jottacloud mountpoint. This page states
what is guaranteed, what is best-effort, and what is not representable.

## Guaranteed

| Operation | Guarantee |
|---|---|
| `getattr` | returns size, mtime, kind; a warm metadata hit never touches the network |
| `readdir` | paged and stable during one open directory handle; entries are cached with a TTL |
| `read` | a warm blob is served from disk with no network request; a cold read fills the cache first and verifies the hash before returning bytes |
| `write` | lands in the cache and the durable spool before the call returns; drains to the cloud in the background |
| `create` / `mkdir` | durable in the spool and journal before success is returned |
| `unlink` / `rmdir` | recorded in the journal, then applied remotely; local trash copy per `sync.trash_retention_days` |
| `rename` | one remote move where possible; both the old and the new path are locked in a deterministic order |
| `truncate` | implemented as a new version (content-addressed store: never in-place) |
| `statfs` | reports the account quota when known, the cache size otherwise |
| open handles | pin their blob for the lifetime of the handle; eviction never removes it |

## Best-effort

- **xattr**: passed through where the backend supports it; `user.*` is stored
  locally and not synchronised.
- **advisory locks** (`flock`/`fcntl`): honoured between processes using this
  mount on this machine; not distributed, not visible to Jottacloud.
- **timestamps**: mtime is what the server reports; atime is local only.
- **permissions**: the mount exposes the files as the mounting user; POSIX mode
  bits are synthesised from a fixed umask (configurable).

## Not representable

- **Hard links**: a Jottacloud object is a single path. `link()` returns
  `EPERM`, documented rather than faked.
- **Special files**: device nodes, FIFOs and sockets are not stored; they are
  skipped by the sync scanner and rejected by `mknod`.
- **Case-only renames on a case-insensitive remote**: detected and reported as a
  conflict rather than silently losing one name.

## Failure behaviour

| Situation | Behaviour |
|---|---|
| offline, blob cached | read succeeds from cache |
| offline, blob not cached | read fails with `EIO` after the configured timeout, never hangs |
| offline, write | succeeds into the spool until `mount.durable_spool_bytes` is full, then `EIO` |
| server 429/503 | the request retries with backoff inside the configured timeout, then `EIO` |
| corrupt blob | deleted, re-fetched once, then `EIO` — a corrupt blob is never served |
| engine panic | the callback returns `EIO`; the mount and the UI stay alive (invariant 7) |
| unmount | in-flight writes drain or are journalled for the next start |
