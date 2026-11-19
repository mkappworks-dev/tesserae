# 04: The seal beside water is computed once per column

Status: done

Blocked by: 02

Spec: [Worldgen performance](../spec.md).

## What to build

The height below which rock beside water stays solid is computed the first time a carve asks for a column and kept in the column cache, one atomic per column, so the Chunks over a Cell and the coarse levels stop asking nine Water levels per carved column each time. Two workers that race compute the same value.

## Acceptance criteria

- [x] Version 3 Chunks match the ticket 01 fingerprint; the committed version 3 digests pass unchanged.
- [x] Generated water has no air below or beside it, and flooded caverns stay sealed from dry tunnels, at 1.0 and 0.5 m.
- [x] Bench figures before and after in Comments.

## Comments

### Bench figures, 2026-11-19

Each binary was built at its commit with `RUSTFLAGS="-D warnings" cargo bench --locked --no-run -p tesserae-core --bench worldgen_v3` and copied out of the target dir; all builds finished before any timing, and nothing else built during the runs. Each round runs every case in a fresh subprocess of each binary, the two alternating which goes first; a case is thread CPU time per Chunk over 3 fresh generators. Figures are medians over 9 rounds, and the ratio is the median of the per-round ratios. Commands run from the directory holding the copied binaries.

Before and after:

`bench-04` (8c54ad8e) against `bench-01` (fcb5db58), `uptime` before `17:46  up 7 days, 16:45, 1 user, load averages: 1.67 3.17 3.99`, after `17:47  up 7 days, 16:46, 1 user, load averages: 1.92 3.06 3.92`:

```sh
TESSERAE_BENCH_ROUNDS=9 TESSERAE_BENCH_AGAINST=$PWD/bench-01 ./bench-04 --bench
```

| Case | µs | 01 µs | Ratio |
|---|---|---|---|
| straddling forest @ 1 m (1 Chunks) | 9412.1 | 7207.6 | 130.6% |
| straddling dry @ 1 m (1 Chunks) | 7068.9 | 3624.8 | 194.9% |
| sky @ 1 m (1 Chunks) | 0.2 | 0.2 | 107.7% |
| deep rock @ 1 m (1 Chunks) | 1168.2 | 1402.2 | 83.3% |
| river @ 1 m (1 Chunks) | 16184.0 | 12835.4 | 126.3% |
| cavern @ 1 m (1 Chunks) | 17568.1 | 4148.5 | 423.9% |
| coarse level 3 @ 1 m (4 Chunks) | 3591.1 | 3978.5 | 90.5% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1325.1 | 5223.6 | 25.4% |
| straddling forest @ 0.5 m (1 Chunks) | 4537.8 | 1231.3 | 368.5% |
| straddling dry @ 0.5 m (1 Chunks) | 4758.1 | 1475.8 | 321.6% |
| sky @ 0.5 m (1 Chunks) | 0.2 | 0.2 | 93.6% |
| deep rock @ 0.5 m (1 Chunks) | 1055.9 | 1273.5 | 82.6% |
| river @ 0.5 m (1 Chunks) | 14976.6 | 1131.8 | 1324.3% |
| cavern @ 0.5 m (1 Chunks) | 15808.7 | 5725.7 | 275.9% |
| coarse level 3 @ 0.5 m (4 Chunks) | 3608.4 | 3919.4 | 91.9% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 821.0 | 1106.6 | 74.2% |

This commit's own effect:

`bench-04` (8c54ad8e) against `bench-03` (3c3e9948), `uptime` before `17:48  up 7 days, 16:47, 1 user, load averages: 1.64 2.76 3.74`, after `17:48  up 7 days, 16:47, 1 user, load averages: 1.60 2.64 3.67`:

```sh
TESSERAE_BENCH_ROUNDS=9 TESSERAE_BENCH_AGAINST=$PWD/bench-03 ./bench-04 --bench
```

| Case | µs | 03 µs | Ratio |
|---|---|---|---|
| straddling forest @ 1 m (1 Chunks) | 9415.5 | 9400.4 | 100.1% |
| straddling dry @ 1 m (1 Chunks) | 7052.3 | 7059.6 | 100.0% |
| sky @ 1 m (1 Chunks) | 0.2 | 0.2 | 93.7% |
| deep rock @ 1 m (1 Chunks) | 1171.5 | 1169.8 | 100.5% |
| river @ 1 m (1 Chunks) | 16188.4 | 16216.1 | 99.8% |
| cavern @ 1 m (1 Chunks) | 17545.8 | 19695.4 | 89.1% |
| coarse level 3 @ 1 m (4 Chunks) | 3605.1 | 3661.8 | 98.4% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1325.1 | 1487.3 | 89.2% |
| straddling forest @ 0.5 m (1 Chunks) | 4543.2 | 4528.7 | 100.4% |
| straddling dry @ 0.5 m (1 Chunks) | 4751.9 | 4738.4 | 100.4% |
| sky @ 0.5 m (1 Chunks) | 0.2 | 0.2 | 99.8% |
| deep rock @ 0.5 m (1 Chunks) | 1053.8 | 1046.0 | 100.6% |
| river @ 0.5 m (1 Chunks) | 14934.2 | 17099.0 | 87.4% |
| cavern @ 0.5 m (1 Chunks) | 15805.5 | 17195.8 | 92.1% |
| coarse level 3 @ 0.5 m (4 Chunks) | 3604.9 | 3761.9 | 96.0% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 822.1 | 865.2 | 95.0% |

Against 03 nothing is slower: cavern 89% (1 m) and 92% (0.5 m), river @ 0.5 m 87%, forest 3x3 columns 89% (1 m) and 95% (0.5 m); the rest within 1%. Against 01 the single-Chunk cases stay slower, as in tickets 02 and 03.

Version 3's fingerprint (an uncommitted 46-line hash of a fixed Chunk grid at 1.0 and 0.5 m, plus queries) is identical to fcb5db58's at every one of 80606a10, 3c3e9948, 8c54ad8e and ac49c2f3, and the committed version 3 digests pass.
