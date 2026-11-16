# Configuration

Related: [deviations](deviations.md)

The configuration is TOML at `<state>/config.toml` (default state directory:
`$XDG_DATA_HOME/nidvue-drive` on Linux, `~/.local/share/nidvue-drive` otherwise,
`NIDVUE_STATE` overrides). Every value has a default in
`crates/nidvue-core/src/config.rs`; the table below is that file, for humans.

## `[cache]`

| Key | Default | Meaning |
|---|---|---|
| `max_size_bytes` | 20 GiB | hard cap on blob bytes |
| `min_free_disk_bytes` | 2 GiB | free-space reserve, never filled |
| `eviction_policy` | `lru_k` | `lru_k`, `lru` or `none` |
| `prefetch` | `true` | prefetch listings and sequential reads |
| `readahead_bytes` | 4 MiB | readahead window |
| `high_watermark` | 0.90 | start evicting above this fraction |
| `low_watermark` | 0.80 | stop evicting below this fraction |

## `[sync]`

| Key | Default | Meaning |
|---|---|---|
| `poll_interval_ms` | 5000 | floor of the remote poll interval |
| `max_poll_interval_ms` | 60000 | ceiling after repeated empty polls |
| `full_scan_interval_ms` | 21600000 | safety-net full rescan (0 disables) |
| `conflict_policy` | `keep_both` | `keep_both`, `prefer_local`, `prefer_remote` |
| `workers` | 4 | transfer/hashing workers |
| `bandwidth_limit_bps` | 0 | aggregate cap, 0 = unlimited |
| `debounce_ms` | 500 | filesystem event coalescing |
| `rename_window_ms` | 5000 | delete+create window treated as a move |
| `max_file_size_bytes` | 0 | refuse larger files, 0 = no limit |
| `bulk_delete_threshold` | 100 | confirmations required above this |
| `trash_retention_days` | 30 | local trash retention, 0 disables |
| `paused` | false | pause every pair |

Each `[[sync.pairs]]` entry has `name`, `local`, `remote`, `paused`,
`include`, `exclude` and an optional `conflict_policy`.

## `[mount]`

| Key | Default | Meaning |
|---|---|---|
| `mountpoint` | `/tmp/nidvue` | where the filesystem appears |
| `read_only` | false | refuse every mutation |
| `allow_other` | false | FUSE `allow_other` (needs `user_allow_other`) |
| `attr_ttl_ms` | 1000 | kernel attribute cache |
| `entry_ttl_ms` | 1000 | kernel entry cache |
| `durable_spool_bytes` | 1 GiB | write-back spool before `EIO` |

## `[network]`

| Key | Default | Meaning |
|---|---|---|
| `base_url` | `https://jfs.jottacloud.com/jfs` | JFS base |
| `allocate_url` | `https://api.jottacloud.com/files/v1` | Allocate API base |
| `device` | `Jotta` | Jottacloud device |
| `mountpoint` | `Archive` | Jottacloud mountpoint |
| `max_concurrency` | 8 | AIMD ceiling |
| `min_concurrency` | 1 | AIMD floor |
| `requests_per_second` | 20 | token bucket rate |
| `burst` | 20 | token bucket burst |
| `timeout_ms` | 60000 | per-request timeout |
| `user_agent` | `nidvue-drive/<version>` | HTTP user agent |

## `[log]` and `[ui]`

| Key | Default | Meaning |
|---|---|---|
| `log.level` | `info` | `error`, `warn`, `info`, `debug`, `trace` |
| `log.json` | false | JSON lines |
| `log.dir` | empty | rotating file log directory |
| `log.max_files` | 7 | rotated files kept |
| `log.otel_endpoint` | none | OTLP/HTTP base URL, e.g. `http://127.0.0.1:4318`; exports to `<endpoint>/v1/metrics` |
| `log.otel_interval_s` | 15 | seconds between OTLP exports |
| `log.prometheus_bind` | none | bind address for the scrape endpoint, e.g. `127.0.0.1:9464` |
| `ui.theme` | `system` | `system`, `dark`, `light` |
| `ui.language` | `en` | BCP-47 tag |
| `ui.start_minimised` | false | start in the tray |

## `[secret]`

| Key | Default | Meaning |
|---|---|---|
| `secret.backend` | `auto` | `auto`, `file` or `keyring` |

`auto` probes the OS keyring (Secret Service on Linux, the Security framework on
macOS, the Credential Manager on Windows) with a throwaway entry. When the probe
answers, tokens live in the keyring. When it does not — a headless session, a
container, no D-Bus — the encrypted file store takes over and `doctor` prints
the platform's own reason. `file` never probes; `keyring` fails instead of
downgrading.

## CLI flags

`nidvue-cli` exposes `--state`, `--json`, and per-command flags (`--token-stdin`,
`--read-only`, `--bench`).

Every key above is also reachable by dot path, which is what closes the long
tail without a flag per key:

```sh
nidvue-cli config path                      # where the file is
nidvue-cli config list                      # every key and its value
nidvue-cli config list --json               # the same, as an object
nidvue-cli config get cache.max_size_bytes  # one leaf
nidvue-cli config set cache.max_size_bytes 1048576
nidvue-cli config set secret.backend keyring
nidvue-cli config set log.prometheus_bind 127.0.0.1:9464
```

The value is parsed against the type already there, the result is validated
before it is written, and an unknown key is an error that names the closest
match. A value that would make the configuration invalid (a zero cache, a
watermark pair out of order) is refused, not written.

## Telemetry

`log.prometheus_bind = "127.0.0.1:9464"` serves:

```sh
curl -s http://127.0.0.1:9464/metrics    # text exposition 0.0.4
curl -s http://127.0.0.1:9464/healthz    # ok
```

`log.otel_endpoint = "http://127.0.0.1:4318"` posts OTLP/HTTP JSON metrics every
`log.otel_interval_s` seconds. Both read the same snapshot, so a scrape and an
export cannot disagree. Neither transport can stop a sync: a collector that is
down is a logged warning. `nidvue-cli doctor` reports which transports are live
and which store holds the credentials.
