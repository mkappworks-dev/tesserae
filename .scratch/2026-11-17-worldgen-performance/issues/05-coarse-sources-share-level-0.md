# 05: The coarse levels share level 0's terrain and columns

Status: done

Blocked by: 02

Spec: [Worldgen performance](../spec.md).

## What to build

A generator can be made to share another's work when its seed and every terrain parameter match: it always adopts the other's terrain, so one drainage solve serves every level, and adopts the column cache only at the same Voxel size. The Server makes its coarse levels' sources share level 0's generator; the level-1 source, level 0 without surface features, shares the column cache too. The direct column generators of the farthest levels stay as they are.

## Acceptance criteria

- [x] A shared source generates the same Chunks as one made alone, at every level, warm or cold.
- [x] Sharing a generator whose seed or terrain differs is refused.
- [x] Version 3 Chunks match the ticket 01 fingerprint; the committed version 3 digests pass unchanged.
- [x] Bench figures and the coarse levels' worker time before and after in Comments.

## Comments

### Bench figures, 2026-11-19

Each binary was built at its commit with `RUSTFLAGS="-D warnings" cargo bench --locked --no-run -p tesserae-core --bench worldgen_v3` and copied out of the target dir; all builds finished before any timing, and nothing else built during the runs. Each round runs every case in a fresh subprocess of each binary, the two alternating which goes first; a case is thread CPU time per Chunk over 3 fresh generators. Figures are medians over 9 rounds, and the ratio is the median of the per-round ratios. Commands run from the directory holding the copied binaries.

Before and after:

`bench-05` (ac49c2f3) against `bench-01` (fcb5db58), `uptime` before `17:47  up 7 days, 16:46, 1 user, load averages: 1.92 3.06 3.92`, after `17:47  up 7 days, 16:46, 1 user, load averages: 1.64 2.87 3.82`:

```sh
TESSERAE_BENCH_ROUNDS=9 TESSERAE_BENCH_AGAINST=$PWD/bench-01 ./bench-05 --bench
```

| Case | µs | 01 µs | Ratio |
|---|---|---|---|
| straddling forest @ 1 m (1 Chunks) | 9388.5 | 7205.5 | 130.3% |
| straddling dry @ 1 m (1 Chunks) | 7046.7 | 3625.2 | 194.5% |
| sky @ 1 m (1 Chunks) | 0.2 | 0.2 | 100.0% |
| deep rock @ 1 m (1 Chunks) | 1166.5 | 1402.3 | 83.3% |
| river @ 1 m (1 Chunks) | 16171.2 | 12827.4 | 125.9% |
| cavern @ 1 m (1 Chunks) | 17565.5 | 4143.5 | 423.8% |
| coarse level 3 @ 1 m (4 Chunks) | 3593.5 | 3985.3 | 90.1% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1319.1 | 5220.7 | 25.3% |
| straddling forest @ 0.5 m (1 Chunks) | 4543.0 | 1230.9 | 367.6% |
| straddling dry @ 0.5 m (1 Chunks) | 4747.9 | 1481.0 | 320.4% |
| sky @ 0.5 m (1 Chunks) | 0.2 | 0.2 | 114.4% |
| deep rock @ 0.5 m (1 Chunks) | 1050.1 | 1276.6 | 82.0% |
| river @ 0.5 m (1 Chunks) | 14980.5 | 1127.3 | 1329.4% |
| cavern @ 0.5 m (1 Chunks) | 15821.6 | 5700.2 | 277.0% |
| coarse level 3 @ 0.5 m (4 Chunks) | 3604.1 | 3932.7 | 91.7% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 819.3 | 1105.7 | 74.0% |

This commit's own effect:

`bench-05` (ac49c2f3) against `bench-04` (8c54ad8e), `uptime` before `17:48  up 7 days, 16:47, 1 user, load averages: 1.60 2.64 3.67`, after `17:49  up 7 days, 16:48, 1 user, load averages: 1.64 2.54 3.59`:

```sh
TESSERAE_BENCH_ROUNDS=9 TESSERAE_BENCH_AGAINST=$PWD/bench-04 ./bench-05 --bench
```

| Case | µs | 04 µs | Ratio |
|---|---|---|---|
| straddling forest @ 1 m (1 Chunks) | 9369.4 | 9421.1 | 99.4% |
| straddling dry @ 1 m (1 Chunks) | 7057.6 | 7059.2 | 100.0% |
| sky @ 1 m (1 Chunks) | 0.2 | 0.2 | 88.8% |
| deep rock @ 1 m (1 Chunks) | 1167.3 | 1171.0 | 99.6% |
| river @ 1 m (1 Chunks) | 16180.2 | 16166.4 | 100.2% |
| cavern @ 1 m (1 Chunks) | 17558.7 | 17598.6 | 99.8% |
| coarse level 3 @ 1 m (4 Chunks) | 3586.0 | 3595.5 | 99.7% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1321.0 | 1322.6 | 100.0% |
| straddling forest @ 0.5 m (1 Chunks) | 4537.6 | 4564.2 | 99.3% |
| straddling dry @ 0.5 m (1 Chunks) | 4742.2 | 4773.4 | 99.3% |
| sky @ 0.5 m (1 Chunks) | 0.2 | 0.2 | 100.0% |
| deep rock @ 0.5 m (1 Chunks) | 1049.6 | 1051.6 | 99.4% |
| river @ 0.5 m (1 Chunks) | 14928.3 | 14931.7 | 100.0% |
| cavern @ 0.5 m (1 Chunks) | 15824.5 | 15772.4 | 100.4% |
| coarse level 3 @ 0.5 m (4 Chunks) | 3597.7 | 3606.4 | 99.8% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 818.0 | 821.9 | 99.5% |

Against 04 every case is within 1% (sky, at 0.2 µs, is timer noise): the bench builds each case's generator alone, so it does not exercise the sharing between levels; the coarse levels' worker time in a flight profile is what shows it.

Version 3's fingerprint (an uncommitted 46-line hash of a fixed Chunk grid at 1.0 and 0.5 m, plus queries) is identical to fcb5db58's at every one of 80606a10, 3c3e9948, 8c54ad8e and ac49c2f3, and the committed version 3 digests pass.

### End flight profile, 2026-11-19

The end, ac49c2f3, against the starting commit, fcb5db58, and the mid-point after ticket 03, 3c3e9948, in a real flight over Terra. Worker CPU, ms per second of flight:

| Level | Stage | Start | Mid | End | End / start | End / mid |
|---|---|---|---|---|---|---|
| 0 | generate | 988.9 (912.4–1000.6) | 254.3 (234.5–267.2) | 188.0 (183.9–199.8) | 19% | 74% |
| 0 | relight | 77.6 (71.0–87.7) | 97.0 (86.1–104.5) | 94.7 (88.8–96.2) | 122% | 98% |
| 0 | mesh | 19.5 (19.1–22.5) | 26.1 (24.0–27.2) | 27.6 (26.9–27.8) | 142% | 106% |
| 1 | generate | 284.1 (267.0–353.1) | 273.2 (264.4–284.7) | 259.6 (251.9–264.6) | 91% | 95% |
| 1 | mesh | 194.0 (187.6–200.7) | 268.5 (238.6–281.8) | 273.8 (264.6–281.5) | 141% | 102% |
| 2 | generate | 63.5 (61.5–131.4) | 52.6 (49.8–53.6) | 50.6 (49.6–51.3) | 80% | 96% |
| 2 | mesh | 2.0 (2.0–2.4) | 1.9 (1.7–2.0) | 1.9 (1.9–2.0) | 95% | 100% |
| 3 | generate | 10.0 (9.5–21.1) | 8.8 (8.7–9.7) | 8.9 (8.5–9.7) | 89% | 101% |
| 3 | mesh | 4.1 (3.8–4.5) | 4.1 (3.6–4.4) | 4.1 (3.9–4.3) | 100% | 100% |
| Coarse (1–5) | generate | 357.6 (338.4–508.1) | 335.4 (322.9–346.9) | 318.8 (310.4–325.6) | 89% | 95% |
| All levels | generate | 1350.2 (1250.8–1489.1) | 590.3 (557.4–602.6) | 510.0 (496.6–522.6) | 38% | 86% |

Level 0's fluid work is 21.8, 32.1 and 33.0 ms/s. The coarse levels' worker time per Chunk, which is what sharing should cut, because the freed workers generate more coarse Chunks in the same 10 s (level 1: 795 at the start, 985 mid, 1001 at the end):

| Level | Start ms/Chunk | Mid ms/Chunk | End ms/Chunk |
|---|---|---|---|
| 1 | 3.57 | 2.77 | 2.58 |
| 2 | 3.80 | 3.19 | 3.07 |
| 3 | 0.60 | 0.54 | 0.54 |

Generation is cheaper in a real flight: level 0 by 81% against the start and 26% against the mid-point (tickets 04 and 05), the coarse levels by 11% per second and 28% per level-1 Chunk against the start. This ticket's own share, mid to end, is small at the coarse levels: 5% per second, 7% per level-1 Chunk and 4% per level-2 Chunk. Level-1 meshing and level-0 relight rise with the extra throughput; they are out of this spec's scope.

How they were taken: each binary was built at its commit with `RUSTFLAGS="-D warnings" cargo build --locked -p tesserae --features profiling --release` (after `git switch --detach <commit>`) and copied out of the target dir into a scratch folder, `$SCRATCH`, as `tesserae-start` (fcb5db58), `tesserae-mid` (3c3e9948) and `tesserae-end` (ac49c2f3); all builds, and a rebuild of Terra's dist with `scripts/build-guests` (its Flora content was stale), finished before any timing. One series is one `scripts/profile-frames` invocation, the `flight` fixture offscreen at 1920x1080 with three repeats, each logging on and off:

```sh
scripts/profile-frames --binary $SCRATCH/tesserae-<point> --profile release \
  --features profiling --fixtures flight --repeats 3 $SCRATCH/runs/r<round>-<point>
```

Three rounds, interleaved by binary: start, mid, end; then end, mid, start; then start, mid, end. Before each series the driver checked `uptime` and waited for no `cargo`, `rustc` or `tesserae` process and no runner lock; it never had to wait. Every run passed the runner's audit (full fixture window, streaming settled before the flight, fixture ready). Figures are worker CPU per second of the 10 s fixture window, from the logging-on runs: the median over 9 runs (3 rounds x 3 repeats), with the range in brackets. Levels 4 and 5 generate nothing in this window after the first start run (2.5 ms/s at level 4 once); level 1 to 3 relight is always 0.

`uptime` over the series, from `17:59 load averages: 3.50 5.22 4.68` at the first to `18:30 load averages: 5.86 6.17 6.11` at the last; the highest one-minute load was 7.10 (before round 3's mid series).
