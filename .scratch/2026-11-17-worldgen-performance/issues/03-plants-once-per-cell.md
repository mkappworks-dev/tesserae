# 03: The Plants on a Cell are decided once

Status: done

Blocked by: 02

Spec: [Worldgen performance](../spec.md).

## What to build

The Plants standing on a Cell's columns are decided once, with the crop fields and footprints that decide them, and kept in a bounded map of compact entries (the feature, the base and the draw). A Chunk takes the Plants of its own and neighbouring Cells within a tree's reach and stands those whose Voxels fall in it, so the ten or so Chunks over a Cell stop deciding the same trees again. Reaching the bound drops the oldest half. Spawning clear of Plants reads the same map.

## Acceptance criteria

- [x] Version 3 Chunks match the ticket 01 fingerprint; the committed version 3 digests pass unchanged.
- [x] A tree reaching across a Chunk border is the same from both Chunks, warm or cold, in any order.
- [x] A generator without Plants fills no entry.
- [x] Bench figures before and after, and the mid-point flight profile, in Comments.

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

### Mid-point flight profile, 2026-11-19

After this ticket, 3c3e9948 (mid), against the starting commit, fcb5db58 (start), in a real flight over Terra. Worker CPU, ms per second of flight:

| Level | Stage | Start | Mid | Mid / start |
|---|---|---|---|---|
| 0 | generate | 988.9 (912.4–1000.6) | 254.3 (234.5–267.2) | 26% |
| 0 | relight | 77.6 (71.0–87.7) | 97.0 (86.1–104.5) | 125% |
| 0 | mesh | 19.5 (19.1–22.5) | 26.1 (24.0–27.2) | 134% |
| 1 | generate | 284.1 (267.0–353.1) | 273.2 (264.4–284.7) | 96% |
| 1 | mesh | 194.0 (187.6–200.7) | 268.5 (238.6–281.8) | 138% |
| 2 | generate | 63.5 (61.5–131.4) | 52.6 (49.8–53.6) | 83% |
| 2 | mesh | 2.0 (2.0–2.4) | 1.9 (1.7–2.0) | 95% |
| 3 | generate | 10.0 (9.5–21.1) | 8.8 (8.7–9.7) | 88% |
| 3 | mesh | 4.1 (3.8–4.5) | 4.1 (3.6–4.4) | 100% |
| Coarse (1–5) | generate | 357.6 (338.4–508.1) | 335.4 (322.9–346.9) | 94% |
| All levels | generate | 1350.2 (1250.8–1489.1) | 590.3 (557.4–602.6) | 44% |

A flight sees the 3x3 column case, not the lone cold Chunk: level-0 generation falls to a quarter, about 15.0 to 3.9 ms of worker CPU per Chunk over the same 660 Chunks; every mid run is below every start run. The freed workers reach more level-1 Chunks in the same window (985 against 795 generated), so level-1 generation per second barely moves while its cost per Chunk falls from 3.6 to 2.8 ms; level-1 meshing, level-0 relight and fluid (21.8 to 32.1 ms/s) rise with that extra throughput, not with generation. Relight and mesh are out of this spec's scope.

How they were taken: each binary was built at its commit with `RUSTFLAGS="-D warnings" cargo build --locked -p tesserae --features profiling --release` (after `git switch --detach <commit>`) and copied out of the target dir into a scratch folder, `$SCRATCH`, as `tesserae-start` (fcb5db58), `tesserae-mid` (3c3e9948) and `tesserae-end` (ac49c2f3); all builds, and a rebuild of Terra's dist with `scripts/build-guests` (its Flora content was stale), finished before any timing. One series is one `scripts/profile-frames` invocation, the `flight` fixture offscreen at 1920x1080 with three repeats, each logging on and off:

```sh
scripts/profile-frames --binary $SCRATCH/tesserae-<point> --profile release \
  --features profiling --fixtures flight --repeats 3 $SCRATCH/runs/r<round>-<point>
```

Three rounds, interleaved by binary: start, mid, end; then end, mid, start; then start, mid, end. Before each series the driver checked `uptime` and waited for no `cargo`, `rustc` or `tesserae` process and no runner lock; it never had to wait. Every run passed the runner's audit (full fixture window, streaming settled before the flight, fixture ready). Figures are worker CPU per second of the 10 s fixture window, from the logging-on runs: the median over 9 runs (3 rounds x 3 repeats), with the range in brackets. Levels 4 and 5 generate nothing in this window after the first start run (2.5 ms/s at level 4 once); level 1 to 3 relight is always 0.

`uptime` over the series, from `17:59 load averages: 3.50 5.22 4.68` at the first to `18:30 load averages: 5.86 6.17 6.11` at the last; the highest one-minute load was 7.10 (before round 3's mid series).
