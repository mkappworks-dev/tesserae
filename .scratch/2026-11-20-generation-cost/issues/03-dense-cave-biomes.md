# 03: A Chunk's cave Biomes in a dense array

Status: done

Blocked by: 01

Spec: [Generation cost](../spec.md).

## What to build

`Worldgen::chunk` keeps the cave Biome of each cave Biome cell it asks in a dense array indexed from the Chunk's origin cell, instead of a SipHash map keyed by `[i32; 3]`. Where a Voxel is at least a cave Biome cell wide, nothing is kept, since no two Voxels share a cell. `cave_biome_at` reads a noise corner cache for its two fields.

## Acceptance criteria

- [x] Version 3 matches the fingerprint at 1.0 and 0.5 m, the crop-field grid included; the committed version 3 digests pass unchanged.
- [x] Cave Biome floors and rock match at 1.0, 0.5 m and at a LOD level whose Voxel is wider than a cell.
- [x] Bench figures against 01's binary in Comments, with `uptime`.

## Comments

### The dense cave Biome cells, 2026-11-20

`CaveBiomeCells` in `underground.rs` replaces the Chunk's `HashMap<[i32; 3], Option<u8>>`: one slot per 4 m cell from the cell of the Chunk's first Voxel to that of its last, each axis sized from `origin + 31 × voxel_size`, the metres the Chunk's last Voxel stands at, so a Voxel size off the powers of two spans whatever cells it reaches. Where a Voxel is 4 m or wider the array is empty and each ask goes straight to the noise. Every ask, kept or not, reads `cave_biome_near`, `cave_biome_at` with an `Fbm3Cache` for each of the cave Climate's two fields; `cave_biome_at` is it with a fresh cache, for fog and the public queries.

Proof: `tests/underground.rs` gains a check that every cave Biome rock and floor Voxel in a column of Chunks a border between two cave Biomes crosses belongs to the cave Biome `cave_biome_index_at` names at its point, at 1 m and 0.5 m, at 0.75 m, and at 8 m, the Server's level-4 source at 1 m. It passes on the old map too; an index that folds two cells into one slot fails it at 0.75 m. The uncommitted fingerprint matches all 52 baseline lines, the coarse-level grids and `grid crops` included; `generator_version` and the tesserae-core tests pass. The figures are the parent's.

### Bench figures, 2026-11-20

`bench-gen03` (this commit, on 5e5d5e0c) against `bench-gen04` (5e5d5e0c), so the ratio is this ticket alone; the figure against 01's binary is ticket 04's times this one. Built and run as in ticket 02, medians over 15 rounds, with other worktrees building: `uptime` before `7:31  up 8 days,  6:30, 1 user, load averages: 20.37 23.61 22.31`, after ` 7:34  up 8 days,  6:32, 1 user, load averages: 24.73 24.91 23.02`.

```sh
TESSERAE_BENCH_ROUNDS=15 TESSERAE_BENCH_AGAINST=$PWD/bench-gen04 ./bench-gen03 --bench
```

| Case | µs | 04 µs | Ratio |
| --- | --- | --- | --- |
| straddling forest @ 1 m (1 Chunks) | 8887.2 | 9935.7 | 99.4% |
| straddling dry @ 1 m (1 Chunks) | 6308.2 | 7731.6 | 90.1% |
| sky @ 1 m (1 Chunks) | 0.4 | 0.4 | 90.9% |
| deep rock @ 1 m (1 Chunks) | 829.5 | 1179.1 | 97.9% |
| river @ 1 m (1 Chunks) | 4790.3 | 5727.9 | 94.1% |
| cavern @ 1 m (1 Chunks) | 7798.7 | 7631.4 | 99.6% |
| coarse level 3 @ 1 m (4 Chunks) | 1085.4 | 1012.5 | 101.3% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1582.8 | 1779.3 | 102.1% |
| straddling crop field @ 1 m (1 Chunks) | 18476.2 | 19919.4 | 99.7% |
| crop field column @ 1 m (4 Chunks) | 5179.5 | 5581.3 | 100.9% |
| straddling forest @ 0.5 m (1 Chunks) | 3404.9 | 4365.4 | 97.6% |
| straddling dry @ 0.5 m (1 Chunks) | 3915.0 | 4719.5 | 97.1% |
| sky @ 0.5 m (1 Chunks) | 0.3 | 0.3 | 108.6% |
| deep rock @ 0.5 m (1 Chunks) | 678.4 | 720.2 | 98.2% |
| river @ 0.5 m (1 Chunks) | 4343.3 | 3955.9 | 103.5% |
| cavern @ 0.5 m (1 Chunks) | 5365.0 | 4558.4 | 98.3% |
| coarse level 3 @ 0.5 m (4 Chunks) | 1185.5 | 1366.1 | 102.5% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 990.9 | 943.3 | 104.6% |
| straddling crop field @ 0.5 m (1 Chunks) | 17317.6 | 17068.0 | 104.0% |
| crop field column @ 0.5 m (4 Chunks) | 5076.5 | 4941.0 | 95.7% |

Every case is within noise (90% to 109%): the bench's World declares no cave Biomes, so its Chunks never read the cache this ticket replaces. Terra's do; the mid-point sampled flight, in ticket 04, measures the hashing it removes.
