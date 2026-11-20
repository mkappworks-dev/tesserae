# 02: Each column's Biome is asked once per Cell

Status: done

Blocked by: 01

Spec: [Worldgen performance](../spec.md).

## What to build

The per-Cell surface cache, renamed so it is no longer called a Stack, also holds each column's Biome, and every Biome a Chunk asks for reads it: Plants, grounds, ores and the cave-scale blend. A column outside the Chunk's Cell builds its neighbour's cache. Looking a Cell up takes a read lock and first checks the last Cell this thread asked; reaching the bound of 1024 Cells drops the oldest half.

## Acceptance criteria

- [x] Version 3 Chunks match the ticket 01 fingerprint at 1.0 and 0.5 m; the committed version 3 digests pass unchanged.
- [x] A Chunk generated after every cache is dropped equals one generated warm, in any order, with Biomes, Plants and ores.
- [x] Reaching the bound keeps the newer half of the Cells.
- [x] Bench figures before and after, against the baseline binary, in Comments.

## Comments

### Bench figures, 2026-11-19

Each binary was built at its commit with `RUSTFLAGS="-D warnings" cargo bench --locked --no-run -p tesserae-core --bench worldgen_v3` and copied out of the target dir; all builds finished before any timing, and nothing else built during the runs. Each round runs every case in a fresh subprocess of each binary, the two alternating which goes first; a case is thread CPU time per Chunk over 3 fresh generators. Figures are medians over 9 rounds, and the ratio is the median of the per-round ratios. Commands run from the directory holding the copied binaries.

Before and after:

`bench-02` (80606a10) against `bench-01` (fcb5db58), `uptime` before `17:45  up 7 days, 16:44, 1 user, load averages: 2.06 3.59 4.20`, after `17:46  up 7 days, 16:44, 1 user, load averages: 1.78 3.35 4.08`:

```sh
TESSERAE_BENCH_ROUNDS=9 TESSERAE_BENCH_AGAINST=$PWD/bench-01 ./bench-02 --bench
```

| Case | µs | 01 µs | Ratio |
|---|---|---|---|
| straddling forest @ 1 m (1 Chunks) | 6247.1 | 7190.8 | 86.9% |
| straddling dry @ 1 m (1 Chunks) | 6183.5 | 3625.8 | 170.9% |
| sky @ 1 m (1 Chunks) | 0.2 | 0.2 | 122.7% |
| deep rock @ 1 m (1 Chunks) | 1177.5 | 1409.4 | 83.8% |
| river @ 1 m (1 Chunks) | 15280.8 | 12811.9 | 119.2% |
| cavern @ 1 m (1 Chunks) | 16465.3 | 4150.1 | 397.6% |
| coarse level 3 @ 1 m (4 Chunks) | 3644.5 | 3973.1 | 91.8% |
| forest 3x3 columns @ 1 m (36 Chunks) | 2199.9 | 5218.9 | 42.1% |
| straddling forest @ 0.5 m (1 Chunks) | 4408.9 | 1229.5 | 358.7% |
| straddling dry @ 0.5 m (1 Chunks) | 4680.5 | 1484.9 | 315.0% |
| sky @ 0.5 m (1 Chunks) | 0.2 | 0.2 | 93.0% |
| deep rock @ 0.5 m (1 Chunks) | 1057.7 | 1275.0 | 83.1% |
| river @ 0.5 m (1 Chunks) | 14908.6 | 1126.6 | 1317.5% |
| cavern @ 0.5 m (1 Chunks) | 15690.6 | 5709.8 | 275.3% |
| coarse level 3 @ 0.5 m (4 Chunks) | 3760.6 | 3927.8 | 95.6% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 935.6 | 1105.6 | 84.6% |

Slower against 01: every single-Chunk case at 0.5 m but deep rock (straddling forest 359%, straddling dry 315%, river 1318%, cavern 275%), and at 1 m straddling dry 171%, river 119% and cavern 398%. Faster: forest 3x3 columns (42% at 1 m, 85% at 0.5 m), deep rock and coarse level 3. Not investigated; a likely cause is that a single Chunk from a fresh generator pays for building its Cell's and the neighbouring Cells' caches alone, while the forest column of 36 Chunks shares them.

Version 3's fingerprint (an uncommitted 46-line hash of a fixed Chunk grid at 1.0 and 0.5 m, plus queries) is identical to fcb5db58's at every one of 80606a10, 3c3e9948, 8c54ad8e and ac49c2f3, and the committed version 3 digests pass.
