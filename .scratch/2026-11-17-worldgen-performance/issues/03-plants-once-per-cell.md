# 03: The Plants on a Cell are decided once

Status: ready-for-agent

Blocked by: 02

Spec: [Worldgen performance](../spec.md).

## What to build

The Plants standing on a Cell's columns are decided once, with the crop fields and footprints that decide them, and kept in a bounded map of compact entries (the feature, the base and the draw). A Chunk takes the Plants of its own and neighbouring Cells within a tree's reach and stands those whose Voxels fall in it, so the ten or so Chunks over a Cell stop deciding the same trees again. Reaching the bound drops the oldest half. Spawning clear of Plants reads the same map.

## Acceptance criteria

- [x] Version 3 Chunks match the ticket 01 fingerprint; the committed version 3 digests pass unchanged.
- [x] A tree reaching across a Chunk border is the same from both Chunks, warm or cold, in any order.
- [x] A generator without Plants fills no entry.
- [ ] Bench figures before and after, and the mid-point flight profile, in Comments.

## Comments

### Bench figures, 2026-11-19

Each binary was built at its commit with `RUSTFLAGS="-D warnings" cargo bench --locked --no-run -p tesserae-core --bench worldgen_v3` and copied out of the target dir; all builds finished before any timing, and nothing else built during the runs. Each round runs every case in a fresh subprocess of each binary, the two alternating which goes first; a case is thread CPU time per Chunk over 3 fresh generators. Figures are medians over 9 rounds, and the ratio is the median of the per-round ratios. Commands run from the directory holding the copied binaries.

Before and after:

`bench-03` (3c3e9948) against `bench-01` (fcb5db58), `uptime` before `17:46  up 7 days, 16:44, 1 user, load averages: 1.78 3.35 4.08`, after `17:46  up 7 days, 16:45, 1 user, load averages: 1.67 3.17 3.99`:

```sh
TESSERAE_BENCH_ROUNDS=9 TESSERAE_BENCH_AGAINST=$PWD/bench-01 ./bench-03 --bench
```

| Case | µs | 01 µs | Ratio |
|---|---|---|---|
| straddling forest @ 1 m (1 Chunks) | 9413.4 | 7202.6 | 130.7% |
| straddling dry @ 1 m (1 Chunks) | 7042.7 | 3625.2 | 194.0% |
| sky @ 1 m (1 Chunks) | 0.2 | 0.2 | 99.8% |
| deep rock @ 1 m (1 Chunks) | 1169.4 | 1409.1 | 82.6% |
| river @ 1 m (1 Chunks) | 16178.9 | 12801.2 | 126.2% |
| cavern @ 1 m (1 Chunks) | 19710.2 | 4157.2 | 475.2% |
| coarse level 3 @ 1 m (4 Chunks) | 3657.4 | 3977.5 | 91.9% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1488.3 | 5230.4 | 28.4% |
| straddling forest @ 0.5 m (1 Chunks) | 4526.7 | 1231.1 | 368.5% |
| straddling dry @ 0.5 m (1 Chunks) | 4743.9 | 1488.1 | 320.6% |
| sky @ 0.5 m (1 Chunks) | 0.2 | 0.2 | 115.1% |
| deep rock @ 0.5 m (1 Chunks) | 1050.5 | 1280.1 | 82.3% |
| river @ 0.5 m (1 Chunks) | 17073.1 | 1127.5 | 1513.9% |
| cavern @ 0.5 m (1 Chunks) | 17309.2 | 5697.2 | 303.0% |
| coarse level 3 @ 0.5 m (4 Chunks) | 3764.6 | 3922.0 | 96.3% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 866.4 | 1104.8 | 78.4% |

This commit's own effect:

`bench-03` (3c3e9948) against `bench-02` (80606a10), `uptime` before `17:47  up 7 days, 16:46, 1 user, load averages: 1.64 2.87 3.82`, after `17:48  up 7 days, 16:47, 1 user, load averages: 1.64 2.76 3.74`:

```sh
TESSERAE_BENCH_ROUNDS=9 TESSERAE_BENCH_AGAINST=$PWD/bench-02 ./bench-03 --bench
```

| Case | µs | 02 µs | Ratio |
|---|---|---|---|
| straddling forest @ 1 m (1 Chunks) | 9389.2 | 6256.0 | 150.1% |
| straddling dry @ 1 m (1 Chunks) | 7058.7 | 6179.7 | 114.0% |
| sky @ 1 m (1 Chunks) | 0.2 | 0.2 | 93.0% |
| deep rock @ 1 m (1 Chunks) | 1168.5 | 1180.2 | 98.4% |
| river @ 1 m (1 Chunks) | 16176.8 | 15288.9 | 105.9% |
| cavern @ 1 m (1 Chunks) | 19729.4 | 16520.1 | 119.3% |
| coarse level 3 @ 1 m (4 Chunks) | 3658.3 | 3651.0 | 100.2% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1490.0 | 2200.1 | 67.7% |
| straddling forest @ 0.5 m (1 Chunks) | 4532.4 | 4404.0 | 102.8% |
| straddling dry @ 0.5 m (1 Chunks) | 4748.8 | 4691.9 | 101.3% |
| sky @ 0.5 m (1 Chunks) | 0.2 | 0.2 | 90.8% |
| deep rock @ 0.5 m (1 Chunks) | 1053.7 | 1063.0 | 98.8% |
| river @ 0.5 m (1 Chunks) | 17047.5 | 14899.2 | 114.4% |
| cavern @ 0.5 m (1 Chunks) | 17265.3 | 15726.6 | 109.5% |
| coarse level 3 @ 0.5 m (4 Chunks) | 3766.6 | 3761.6 | 100.0% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 866.0 | 936.7 | 92.4% |

Slower against 02: straddling forest @ 1 m 150%, straddling dry @ 1 m 114%, cavern 119% (1 m) and 110% (0.5 m), river 106% (1 m) and 114% (0.5 m). Faster: forest 3x3 columns, 68% at 1 m and 92% at 0.5 m. Against 01 the forest column is 28% at 1 m, while the single-Chunk cases stay slower, as in ticket 02. Not investigated; a likely cause is that a single Chunk from a fresh generator pays for building its Cell's and the neighbouring Cells' caches alone, while the forest column of 36 Chunks shares them.

Version 3's fingerprint (an uncommitted 46-line hash of a fixed Chunk grid at 1.0 and 0.5 m, plus queries) is identical to fcb5db58's at every one of 80606a10, 3c3e9948, 8c54ad8e and ac49c2f3, and the committed version 3 digests pass.
