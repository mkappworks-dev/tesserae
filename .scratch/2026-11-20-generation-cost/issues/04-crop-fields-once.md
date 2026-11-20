# 04: Each crop-field Lattice cell is decided once

Status: done

Blocked by: 01

Spec: [Generation cost](../spec.md).

## What to build

Whether a Lattice cell of a Biome's crop-field entry holds a field, and the first drawn place the ground takes, is decided once per `(Biome, entry, cell)` and kept in a bounded map like `CellMap`: a read-write lock, the oldest half dropped at the bound. `fields_near`, and so every Chunk, `cell_plants` and `plants_clear_m`, read it, including the test that a later entry gives way to an earlier one it crosses. `field_accepts` is unchanged. `forget_caches` clears it.

## Acceptance criteria

- [x] Version 3 matches the fingerprint at 1.0 and 0.5 m, the crop-field grid included; the committed version 3 digests pass unchanged.
- [x] A field crossing a Chunk border is the same from both Chunks, cold or warm, in any order; a Chunk after `forget_caches` equals one generated warm.
- [x] Reaching the bound keeps the newer half.
- [x] Bench figures against 01's binary in Comments, with `uptime`.
- [x] The mid-point flight figures and sampled symbols in Comments.

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

### The mid-point flight, 2026-11-20

The build holds tickets 01 to 04; against the starting commit:

| Fixture | Level | Stage | 4e78a50f | This build | Ratio |
| --- | --- | --- | --- | --- | --- |
| `flight` | 0 | generate | 185.6 | 59.2 | 32% |
| `flight` | 0 | relight | 90.8 | 103.2 | 114% |
| `flight` | 0 | mesh | 27.1 | 27.6 | 102% |
| `flight` | 1 | generate | 249.4 | 220.6 | 88% |
| `flight` | 1 | mesh | 271.7 | 276.0 | 102% |
| `flight` | 2 | generate | 48.3 | 34.1 | 71% |
| `flight` | 3 | generate | 8.5 | 4.7 | 56% |
| `sprint-flight` | 0 | generate | 565.7 | 242.5 | 43% |
| `sprint-flight` | 0 | relight | 403.8 | 402.4 | 100% |
| `sprint-flight` | 0 | mesh | 38.7 | 39.9 | 103% |
| `sprint-flight` | 1 | generate | 797.1 | 611.7 | 77% |
| `sprint-flight` | 1 | mesh | 151.0 | 183.7 | 122% |
| `sprint-flight` | 2 | generate | 255.5 | 163.6 | 64% |
| `sprint-flight` | 3 | generate | 18.9 | 9.4 | 50% |
| `teleport` | 0 | generate | 470.1 | 369.3 | 79% |
| `teleport` | 0 | relight | 87.1 | 88.2 | 101% |
| `teleport` | 0 | mesh | 34.4 | 33.1 | 96% |
| `teleport` | 1 | generate | 490.3 | 369.8 | 75% |
| `teleport` | 1 | mesh | 132.1 | 137.9 | 104% |
| `teleport` | 2 | generate | 424.7 | 264.8 | 62% |
| `teleport` | 3 | generate | 63.4 | 30.1 | 47% |

The teleport's level 0 settles in 2.08 s (1.99 to 2.17) against 2.53 s (2.50 to 2.65).

How they were taken: `tesserae-head` (4e78a50f) and `tesserae-fill01` (fded14c8: tickets 01 to 04 of this spec and the sister spec's 01, its instrument off) were built with `RUSTFLAGS="-D warnings" cargo build --locked -p tesserae --features profiling --release` and copied out; nothing else built or ran a `tesserae` process during the series. Two interleaved rounds (this build, then 4e78a50f; 4e78a50f, then this build), one `scripts/profile-frames --fixtures flight sprint-flight teleport --repeats 3` series each, so six logging-on runs per build and fixture; worker CPU in ms per second of the fixture, medians. `uptime` from `7:55 load averages: 3.97 9.93 15.70` before the first series to `8:35 load averages: 5.76 5.80 6.39` after the last. A third round is taken with the end figures.

Top symbols inside generation, sampled:

Samples inside generation: level 0 2416 (against 8876 at 4e78a50f, -73%), levels 1–5 5804 (against 7636, -24%).

| Symbol | Level 0 | Levels 1–5 | All |
| --- | --- | --- | --- |
| `OreField::at` | 12.0% | 22.6% | 19.5% |
| `tesserae_core::noise::fbm3_with` | 16.6% | 16.9% | 16.8% |
| `Worldgen::cavern_open` | 15.9% | 12.4% | 13.5% |
| `Drainage::wet` | 16.7% | 11.2% | 12.8% |
| `Worldgen::sealed_below_m` | 1.9% | 9.1% | 7.0% |
| `Worldgen::claims_near` | 17.3% | 0.0% | 5.1% |
| `Worldgen::column_cache` | 1.7% | 6.4% | 5.0% |
| `Worldgen::build_columns` | 1.3% | 5.9% | 4.6% |
| `CaveBiomeCells::get` | 2.2% | 5.3% | 4.4% |
| `Worldgen::ground_at` | 7.7% | 2.0% | 3.7% |
| `Worldgen::plants_on` | 12.0% | 0.0% | 3.5% |
| `Terrain::columns_x4` | 1.0% | 4.5% | 3.5% |

The crop fields fell from 67.5% of level 0's generation to 5.9%, and the `[i32; 3]` hashing (11.4% of levels 1–5) is gone. At 4e78a50f the same sampled series gives the table in ticket 01.

The samples: one `scripts/profile-frames --fixtures flight --repeats 3` series of each build, each game process sampled 19.5 s after it appeared, for 12 s (`sample <pid> 12 1 -mayDie`), demangled with `rustfilt`; six processes per build, `uptime` from `8:35 load averages: 5.76` to `8:42 load averages: 5.79`. Generation is every sample at or below `Worldgen::chunk` on a worker; a coarse one is under `lod::generate_coarse`. Inclusive shares; iterator adapters, cache wrappers and unwinder frames left out; a generic symbol is named without its parameters.
