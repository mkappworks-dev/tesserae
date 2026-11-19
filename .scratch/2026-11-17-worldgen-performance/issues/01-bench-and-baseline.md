# 01: A version 3 bench and the starting figures

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Worldgen performance](../spec.md).

## What to build

A bench that times version 3's Chunks as interleaved CPU time, in the style of the Structures bench, over a synthetic World shaped like Terra: a forest Biome with a clumped tree, a dry Biome with its own ground and cave scale, and the fixture's ores, water and caves. Its Chunks, each found by a fixed scan: straddling, the sky and deep-rock early-outs, a river, a cavern, a coarse level-3 Chunk and a whole column of Chunks under the forest. A binary of the starting commit is kept for every later A/B. An uncommitted fingerprint of a fixed grid of version 3 Chunks at 1.0 and 0.5 m is recorded, against which every later ticket proves it changed nothing.

## Acceptance criteria

- [x] The bench runs every case interleaved and prints medians of thread CPU time and the load it ran under.
- [ ] The starting bench figures and the starting flight profile (`scripts/profile-frames`, three interleaved rounds, worker time per level and stage) are in this ticket's Comments, with `uptime` and how they were taken.
- [x] The starting fingerprint is kept with the baseline binary, outside the commit.
- [x] No generator code changes.

## Comments

### Bench figures, 2026-11-19

The baseline's own medians, `bench-01` (fcb5db58) alone. Each binary was built at its commit with `RUSTFLAGS="-D warnings" cargo bench --locked --no-run -p tesserae-core --bench worldgen_v3` and copied out of the target dir; all builds finished before any timing, and nothing else built during the runs. Each round runs every case in a fresh subprocess of each binary, the two alternating which goes first; a case is thread CPU time per Chunk over 3 fresh generators. Figures are medians over 9 rounds, and the ratio is the median of the per-round ratios. Commands run from the directory holding the copied binaries.

`uptime` before `17:45  up 7 days, 16:44, 1 user, load averages: 2.27 3.70 4.25`, after `17:45  up 7 days, 16:44, 1 user, load averages: 2.06 3.59 4.20`.

```sh
TESSERAE_BENCH_ROUNDS=9 ./bench-01 --bench
```

| Case | µs |
|---|---|
| straddling forest @ 1 m (1 Chunks) | 7206.4 |
| straddling dry @ 1 m (1 Chunks) | 3617.4 |
| sky @ 1 m (1 Chunks) | 0.2 |
| deep rock @ 1 m (1 Chunks) | 1406.1 |
| river @ 1 m (1 Chunks) | 12773.5 |
| cavern @ 1 m (1 Chunks) | 4131.3 |
| coarse level 3 @ 1 m (4 Chunks) | 3957.1 |
| forest 3x3 columns @ 1 m (36 Chunks) | 5212.6 |
| straddling forest @ 0.5 m (1 Chunks) | 1228.7 |
| straddling dry @ 0.5 m (1 Chunks) | 1486.2 |
| sky @ 0.5 m (1 Chunks) | 0.2 |
| deep rock @ 0.5 m (1 Chunks) | 1270.7 |
| river @ 0.5 m (1 Chunks) | 1122.8 |
| cavern @ 0.5 m (1 Chunks) | 5687.3 |
| coarse level 3 @ 0.5 m (4 Chunks) | 3897.6 |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 1103.6 |
