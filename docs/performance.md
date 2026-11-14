# Performance budgets

These are budgets, not claims. `cargo bench` and `nidvue-cli doctor --bench`
measure them on the machine that runs them; the numbers recorded below are from
the reference machine in `benches/README.md`. A budget that is missed is either
fixed or recorded in [`deviations.md`](deviations.md) with the measured number.

| Budget | Target (p50) | How it is measured |
|---|---|---|
| Warm cached read, 1 MiB | ≤ 15 ms | `benches/cache_read.rs`, blob present, no network |
| Cold read, 1 MiB on a 50 Mbit link | ≤ 400 ms | fake Jotta server with a 50 Mbit shaper |
| Cached metadata op (`getattr`, `readdir` page) | ≤ 2 ms | `benches/metadata.rs` against a 1M-entry index |
| UI frame | ≤ 16 ms, no frame blocked by engine work | QML frame timer, engine on its own threads |
| Sustained transfer | ≥ 80 % of measured link capacity | `doctor --bench --link-mbps N` |
| Initial sync, 100k small files | bounded memory, resumable | `tests/scale/` with a memory ceiling |
| 1M-entry index | RSS ≤ 300 MB, flat listing latency | `benches/index_scale.rs` |
| Eviction while active | ≤ 5 % CPU | sampled over a 60 s transfer |

## Method

- `p50` is the median of at least 200 samples after 20 warm-up iterations;
  `p99` is reported alongside so a fat tail is visible.
- Cold-read measurements use the fake Jotta server, not the real service, so
  the number is reproducible and not a statement about Jottacloud's CDN.
- Link capacity is measured first (a single large transfer with no cap), then the
  sustained rate is expressed as a fraction of that measurement, never of the
  nominal link speed.
- Memory is sampled from `/proc/self/status` (Linux) or the platform equivalent
  every 500 ms; the peak is reported, not the final value.

## Running

```sh
cargo bench
cargo run -p nidvue-cli --release -- doctor --bench
cargo run -p nidvue-cli --release -- doctor --bench --json > bench.json
```

The `doctor` output includes the reference numbers, the measured numbers, and
pass/fail per budget, so a regression is visible without a dashboard.
