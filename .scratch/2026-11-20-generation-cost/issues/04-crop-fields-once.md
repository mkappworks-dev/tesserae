# 04: Each crop-field Lattice cell is decided once

Status: ready-for-agent

Blocked by: 01

Spec: [Generation cost](../spec.md).

## What to build

Whether a Lattice cell of a Biome's crop-field entry holds a field, and the first drawn place the ground takes, is decided once per `(Biome, entry, cell)` and kept in a bounded map like `CellMap`: a read-write lock, the oldest half dropped at the bound. `fields_near`, and so every Chunk, `cell_plants` and `plants_clear_m`, read it, including the test that a later entry gives way to an earlier one it crosses. `field_accepts` is unchanged. `forget_caches` clears it.

## Acceptance criteria

- [x] Version 3 matches the fingerprint at 1.0 and 0.5 m, the crop-field grid included; the committed version 3 digests pass unchanged.
- [x] A field crossing a Chunk border is the same from both Chunks, cold or warm, in any order; a Chunk after `forget_caches` equals one generated warm.
- [x] Reaching the bound keeps the newer half.
- [x] Bench figures against 01's binary in Comments, with `uptime`.
- [ ] The mid-point flight figures and sampled symbols in Comments.

## Comments

- Built: `Worldgen::cell_field` decides a `(Biome, entry, Lattice cell)` once (held, then the first drawn place `field_accepts` takes) into `Worldgen::fields`, a `CellMap` now generic over its key, bounded at 4096. `entry_fields` reads it, so every Chunk, `cell_plants`, `plants_clear_m` and the "gives way" test do. `forget_caches` clears it.
- `a_field_across_a_chunk_border_is_the_same_warm_cold_and_in_any_order` (tests/fields.rs): the bench's crop-field World, a 3x3 patch round `crop_field_column` at 1.0 and 0.5 m, with crops either side of a border; forward warm equals backward and odd-first on fresh generators, and backward with `forget_caches` before each Chunk on the warm one.
- `only_a_generator_that_plants_fields_keeps_them_until_it_forgets` (worldgen.rs): a grassland with a crop field fills the map and `forget_caches` empties it; without one it stays empty.
- `a_map_keyed_by_field_cell_keeps_the_newer_half_whatever_the_key_order` (cells.rs), beside the existing bound tests.
- Fingerprint: all 52 baseline lines unchanged, both crop-field grids included; `generator_version` passes.

### Bench figures, 2026-11-20

`bench-gen04` (this commit, on 50a6cd68) against `bench-gen01` (537c8f4a), so the figures hold ticket 02 too; built and run as in ticket 02, medians over 15 rounds. Other worktrees were building: `uptime` before `7:25  up 8 days,  6:24, 1 user, load averages: 25.92 30.35 23.92`, after ` 7:26  up 8 days,  6:25, 1 user, load averages: 10.89 24.50 22.29`.

```sh
TESSERAE_BENCH_ROUNDS=15 TESSERAE_BENCH_AGAINST=$PWD/bench-gen01 ./bench-gen04 --bench
```

| Case | µs | 01 µs | Ratio |
| --- | --- | --- | --- |
| straddling forest @ 1 m (1 Chunks) | 6719.0 | 7962.7 | 85.4% |
| straddling dry @ 1 m (1 Chunks) | 4777.2 | 5973.9 | 80.0% |
| sky @ 1 m (1 Chunks) | 0.2 | 0.2 | 100.2% |
| deep rock @ 1 m (1 Chunks) | 730.2 | 1055.4 | 69.3% |
| river @ 1 m (1 Chunks) | 3422.3 | 4536.4 | 76.1% |
| cavern @ 1 m (1 Chunks) | 4848.2 | 5933.5 | 81.8% |
| coarse level 3 @ 1 m (4 Chunks) | 842.6 | 929.1 | 90.7% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1101.8 | 1231.2 | 89.5% |
| straddling crop field @ 1 m (1 Chunks) | 13097.6 | 22249.4 | 59.0% |
| crop field column @ 1 m (4 Chunks) | 3683.1 | 8014.2 | 45.8% |
| straddling forest @ 0.5 m (1 Chunks) | 2763.6 | 3327.8 | 83.0% |
| straddling dry @ 0.5 m (1 Chunks) | 3067.2 | 3595.7 | 85.2% |
| sky @ 0.5 m (1 Chunks) | 0.2 | 0.2 | 93.3% |
| deep rock @ 0.5 m (1 Chunks) | 610.8 | 927.9 | 65.6% |
| river @ 0.5 m (1 Chunks) | 2701.9 | 3205.9 | 84.3% |
| cavern @ 0.5 m (1 Chunks) | 3597.9 | 4103.4 | 87.6% |
| coarse level 3 @ 0.5 m (4 Chunks) | 859.7 | 945.6 | 91.3% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 644.5 | 739.6 | 87.5% |
| straddling crop field @ 0.5 m (1 Chunks) | 11560.1 | 14590.2 | 79.3% |
| crop field column @ 0.5 m (4 Chunks) | 3516.6 | 4797.6 | 73.3% |

The crop-field cases move most: a column of four Chunks over a field costs 46% of 01's at 1 m and 73% at 0.5 m, against 93% and 84% with ticket 02 alone. A lone crop-field Chunk still decides its fields once, so it gains less (59% and 79%); the Chunks over the same Cell share them.
