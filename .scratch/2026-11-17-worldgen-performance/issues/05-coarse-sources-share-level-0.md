# 05: The coarse levels share level 0's terrain and columns

Status: ready-for-agent

Blocked by: 02

Spec: [Worldgen performance](../spec.md).

## What to build

A generator can be made to share another's work when its seed and every terrain parameter match: it always adopts the other's terrain, so one drainage solve serves every level, and adopts the column cache only at the same Voxel size. The Server makes its coarse levels' sources share level 0's generator; the level-1 source, level 0 without surface features, shares the column cache too. The direct column generators of the farthest levels stay as they are.

## Acceptance criteria

- [x] A shared source generates the same Chunks as one made alone, at every level, warm or cold.
- [x] Sharing a generator whose seed or terrain differs is refused.
- [x] Version 3 Chunks match the ticket 01 fingerprint; the committed version 3 digests pass unchanged.
- [ ] Bench figures and the coarse levels' worker time before and after in Comments.

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
