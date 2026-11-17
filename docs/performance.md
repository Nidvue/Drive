# Performance budgets

Related: [Costs](Team/Costs.md) · [Risks](Team/Risks.md) · [deviations](deviations.md)

These are budgets, not claims. `cargo bench` and `nidvue-cli doctor --bench`
measure them on the machine that runs them. A budget that is missed is either
fixed or recorded in [`deviations.md`](deviations.md) with the measured number.

## Measured

Reference machine: Linux x86_64, 24 cores, local NVMe, Rust 1.99 release build.
`cargo bench -p nidvue-benches`, 2026-11-14:

| Budget | Target (p50) | Measured | Verdict |
|---|---|---|---|
| Warm cached read, 1 MiB | ≤ 15 ms | **88 µs** | pass |
| Cached metadata op (`readdir` page, 1000 entries) | ≤ 2 ms | **290 µs** | pass |
| Same page on a 1,000,000-entry index | flat | **270 µs** (dir0 vs dir999) | pass |
| Three-way plan | — | **1.44 µs/path** (100k paths in 144 ms) | — |

Against an emulated 50 Mbit/s link, `cargo bench -p nidvue-benches --bench
transfer` (2026-11-16). The link is the bench's own input — the synthetic remote
serves bytes at a fixed rate — so the fraction is measured against a link whose
capacity is known exactly, not against a nominal one:

| Budget | Target | Measured | Verdict |
|---|---|---|---|
| Cold read, 1 MiB on a 50 Mbit/s link | ≤ 400 ms | **189 ms** | pass |
| Sustained transfer | ≥ 80 % of link capacity | **86.8 %** (8 MiB in 64 objects, 5.18 MiB/s) | pass |
| Warm read after the cold read | ≥ 100× faster | **274×** (690 µs) | pass |
| Peak RSS during the transfer benches | — | **8.1 MiB** | — |

A 100k-path initial sync, `cargo bench -p nidvue-benches --bench initial_sync`
(2026-11-16). This measures the engine's half — scan, index, plan — not 100k
network transfers:

| Step | Measured | Per path |
|---|---|---|
| `scan_local` over 100 000 files | **664 ms** | 6.6 µs |
| index 100 000 rows | **861 ms** | 8.6 µs |
| first three-way plan over an empty remote (100 100 actions) | **57 ms** | 0.57 µs |
| peak RSS | **54 MiB** | 566 bytes |

Bounded memory is the claim, and 566 bytes per path is the number behind it: the
index lives in SQLite, not in RAM.

Still to be measured, and therefore listed in `docs/deviations.md` rather than
claimed here:

| Budget | Target (p50) | Why it is open |
|---|---|---|
| Eviction while active | ≤ 5 % CPU | needs a sustained sync against a full cache, which needs the link above |

### UI frame

| What | Budget | Measured | Command |
|---|---|---|---|
| One rendered frame of the dashboard, 1040×560 | ≤ 16 ms p99 | p50 0.039 ms · p99 0.303 ms · max 0.682 ms | `cargo xtask ui-frames` |

Measured by rendering the live window 300 times and timing each render
(`QQuickWindow::grabWindow`, Qt's software rasteriser, offscreen platform). It is
in the gate: `cargo xtask ui-frames` fails when p99 passes 16 ms or when the
render produced no image. Two limits are stated rather than implied: the
rasteriser is the software one, so this is an upper bound and not a GPU
measurement, and it measures the cost of a frame, not a display's refresh. A
frame cannot be blocked by engine work by construction: every engine call runs on
the worker thread (`apps/nidvue-ui/bridge.h`), and the UI thread only reads the
values the worker published.

An earlier attempt timed `QQuickWindow::afterRendering` under the offscreen
platform and reported 5 µs per frame. It was wrong: that platform fires the frame
signals without rasterising, and scaling the surface 16× did not move the number.
`grabWindow` forces the paint, and its cost does scale with the surface (0.034 ms
at 1040×560 against 8.897 ms at 4160×2240), which is what makes the figure a
measurement rather than an artefact.

## Method

- `p50` is the median of at least 200 samples after warm-up; `p99` is reported
  alongside so a fat tail is visible.
- Cold-read measurements use the fake Jotta server, not the real service, so the
  number is reproducible and not a statement about Jottacloud's CDN.
- Link capacity is measured first (one large uncapped transfer), then the
  sustained rate is expressed as a fraction of that measurement, never of the
  nominal link speed.
- Memory is sampled from `/proc/self/status` (Linux) or the platform equivalent
  every 500 ms; the peak is reported, not the final value.

## Running

```sh
cargo bench -p nidvue-benches
cargo bench -p nidvue-benches --bench transfer
cargo bench -p nidvue-benches --bench initial_sync
cargo run -p nidvue-cli --release -- doctor --bench
cargo run -p nidvue-cli --release -- doctor --bench --json > bench.json
```

`doctor` prints the reference numbers, the measured numbers and pass/fail per
budget, so a regression is visible without a dashboard.

## A fix this found

The metadata page was 49 ms, not 2 ms: `list_dir` used `LIKE ?1 || '%'`, and
SQLite cannot use an index for a pattern built from an expression, so every
listing scanned a million rows. Replacing it with a primary-key range scan
(`path >= prefix AND path < prefix_upper_bound`) took the same page to 290 µs.
The budget caught it; the code change is in `crates/nidvue-cache/src/index.rs`.
