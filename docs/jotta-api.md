# The Jottacloud API surface Nidvue Drive uses

> Unofficial notes, written from observed behaviour of the official clients and
> the public `rclone` backend. Jottacloud publishes no formal API reference;
> anything here can change without notice. Only documented, client-observed
> endpoints are used — no private or reverse-engineered-only endpoint is relied
> on. See `docs/deviations.md` for what that costs.

## 1. Authentication

A **Personal Login Token** is generated in the Jottacloud web UI
(Settings → Security). It is a single-use, base64url-encoded JSON object:

```json
{ "well_known_link": "https://…/.well-known/openid-configuration",
  "username": "…",
  "auth_token": "…" }
```

Exchange it for a bearer token:

1. `GET <well_known_link>` → OIDC discovery document, read `token_endpoint`.
2. `POST <token_endpoint>` as `application/x-www-form-urlencoded`:

   | field | value |
   |---|---|
   | `client_id` | `jottacli` (the identifier Jottacloud uses for command-line clients) |
   | `grant_type` | `password` |
   | `username` | `username` from the login token |
   | `password` | `auth_token` from the login token |
   | `scope` | `openid offline_access` |

3. The response is a standard OIDC token response: `access_token`,
   `refresh_token`, `expires_in` (typically 3600 s), `token_type`.

Refresh: `POST <token_endpoint>` with `grant_type=refresh_token&client_id=jottacli&refresh_token=…`.
Nidvue also refreshes **on a 401**, once, with single-flight, and retries the
original request; a refresh token is rotated and persisted before the retry.

Persistence: both tokens live in `credentials.enc`, ChaCha20-Poly1305 encrypted,
`0600`, never in the config, never in a log line (`SecretString` cannot be
formatted).

## 2. JFS: the file tree

Base: `https://jfs.jottacloud.com/jfs/<username>/<device>/<mountpoint>/<path>`
with `device=Jotta` and `mountpoint=Archive` by default. All requests carry
`Authorization: Bearer <access_token>`.

### 2.1 Listing — always `liststream`

`GET …/<dir>?mode=liststream` streams XML as it is produced: `<file>` and
`<folder>` elements, each with a `path` element relative to the mountpoint, then
a `<stats>` summary. Time-to-first-byte and large-tree behaviour are far better
than `mode=list`, which returns one document per directory level.

```xml
<file path="Photos" filename="a.jpg" md5="…" size="12345"
      modified="2026-11-14T10:15:00.123Z" created="…"/>
<folder path="Photos/2026"/>
<stats folders="12" files="340"/>
```

Nidvue compares the `<stats>` counts with what it actually parsed and fails the
listing as partial when they disagree — a partial listing must never look like a
set of deletions.

### 2.2 Stat, mkdir, delete, move

| Operation | Request |
|---|---|
| stat | `GET …/<path>` (XML `<folder>` or `<file>` with `<currentRevision>`) |
| mkdir | `POST …/<path>` with `<folder><path>…</path></folder>` |
| delete | `DELETE …/<path>` (moves to Jottacloud Trash) |
| move/rename | `POST …/<path>` with a `<file>`/`<folder>` body naming the new path |
| quota | `GET https://api.jottacloud.com/account/v1/customer` (JSON) |

`<currentRevision>` carries `number`, `state` (`COMPLETED`), `size`, `md5`,
`created`, `modified`, `updated`. `state != COMPLETED` means an upload is still
in flight server-side; Nidvue treats such an object as not-yet-visible and does
not download or delete it.

Timestamps use two forms: the historical `2018-07-24-T20:41:10Z` (note the
extra dash) and RFC 3339, sometimes with milliseconds. `nidvue_core::time`
parses both.

## 3. Uploads: the Allocate API, not chunked JFS

Uploads do **not** go through the JFS API. They are allocated first:

```http
POST https://api.jottacloud.com/files/v1/allocate
Content-Type: application/json

{ "bytes": 12345, "md5": "<hex>", "created": "…", "modified": "…",
  "path": "/Jotta/Archive/Documents/report.pdf" }
```

Response:

```json
{ "upload_id": "…", "upload_url": "https://…", "resume_pos": 0,
  "bytes": 12345, "state": "…", "path": "…", "name": "…" }
```

- **201 Created** means the content already exists (MD5 de-duplication): the
  client skips the upload entirely and only the new path is created.
- **resume_pos > 0** means an earlier upload was interrupted; the client sends
  only the remaining suffix.
- The body is sent to `upload_url`, which is a short-lived, pre-signed URL.
- After commit, the client re-reads the object's `<currentRevision>` and compares
  `size` and `md5` with what it sent. A mismatch is a failed transfer, not a
  success with a warning.

Because allocation needs the MD5 before the body is sent, Nidvue hashes the file
first (streaming, `rayon` for CPU-bound hashing, never on the async reactor).

## 4. Rate limits and errors

- 429 and 503 occur under load. `Retry-After` is honoured when present.
- Nidvue keeps a token bucket per host, backs off exponentially with full
  jitter, coalesces identical in-flight requests, and adapts concurrency with
  AIMD between `network.min_concurrency` and `network.max_concurrency`.
- Error bodies are XML (`<error><code>…<message>…`) or JSON depending on the
  endpoint; both are mapped to `NidvueError`, never surfaced raw.
- 401 → one refresh and one retry. A second 401 is `Auth` and stops the lane.

## 5. What does not exist

- **No webhooks, no change feed, no long-poll.** Inbound changes are found by
  polling `liststream` on a budgeted, adaptive interval. The UI says so.
- **No server-side search or delta token.** The scan cursor is a local record of
  what was seen and when; a directory whose listing is unchanged costs one
  request, not one per file.
- **No official API documentation or support.** A breaking change is a client
  bug report, not a support case. The API layer is isolated in `nidvue-jotta` so
  that a change is one crate, not the workspace.
