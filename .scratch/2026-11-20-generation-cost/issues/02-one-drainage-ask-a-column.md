# 02: One drainage ask answers a column's cut, Water level and Biome

Status: done

Blocked by: 01

Spec: [Generation cost](../spec.md).

## What to build

`Drainage::wet`'s Water level does not read the ground height, so one call at the base height answers both the valley cut and the Water level of a raw column, in `build_columns` (four-wide) and in `contained_at_m`. Each column's Biome is decided while the cache is built, whenever the generator has Biomes, and stored in the byte the cache already keeps for it; it, the sea-floor rock and the overhang reach are decided from the fields, base height and cut the raw pass computed, rather than `site` and `seabed_rock` asking them again.

## Acceptance criteria

- [x] Version 3 matches the fingerprint at 1.0 and 0.5 m, the crop-field grid included; the committed version 3 digests pass unchanged.
- [x] A test over many seeds and points: the fused raw column and Biome equal the separate asks bit for bit, four-wide and scalar.
- [x] A Chunk after `forget_caches` equals one generated warm.
- [x] The column cache holds no more memory than before.
- [x] Bench figures against 01's binary in Comments, with `uptime`.

## Comments

### One drainage ask a column, 2026-11-20

`Terrain::column` and `columns_x4` ask a column's base height and then `Drainage::wet` once at it, which answers both the valley cut and the inland Water level, and keep what the base height's samples already hold: the warped point, the continent and erosion noise and the mountain field. `site_of`, `seabed_rock_of` and `overhang_amp_x4_of` read that `Column` rather than warp and sample the fields again; `site` itself is now `site_of(column)`, so `biome_at` asks the base noise once instead of twice. `build_columns` keeps its raw pass's `Column`s and decides each inner column's overhang reach, sea-floor rock and, whenever the generator has Biomes, its Biome from them, storing the Biome in the existing `biomes` byte; a generator without Biomes leaves it `UNASKED` and asks lazily as before. `contained_at_m` asks its own column once, and a neighbour that passes the level gate reuses the level it was gated on. `surface_top_m` reads the same one-ask column. `ColumnCache` gains no field.

Proof: `drainage::tests::the_water_level_is_the_same_whatever_ground_is_asked_about` (seven grounds from `f32::MIN` to 1e6 at 102,400 points over four regions, over a thousand wet and cut); `terrain::tests::one_ask_of_a_column_is_its_separate_asks_bit_for_bit` (8 seeds, 2,560 columns each, four-wide and scalar, against the height, inland water, site and sea-floor rock asked apart and the scalar overhang reach, with cuts, water, rock and overhangs all crossed); `worldgen::tests::a_cells_biomes_are_decided_with_its_columns_as_biome_at_decides_them` (1, 0.5 and 0.3 m, four seeds, four Cells each, and the lazy path without Biomes); `worldgen_caches` for a Chunk after `forget_caches` over every bench case, the crop-field ones included. The uncommitted fingerprint matches all 52 baseline lines and `generator_version` passes unchanged. The figures are the parent's.

### Bench figures, 2026-11-20

`bench-gen02` (this commit) against `bench-gen01` (537c8f4a), each built with `RUSTFLAGS="-D warnings" cargo bench --locked --no-run -p tesserae-core --bench worldgen_v3` and copied out; interleaved rounds, thread CPU time per Chunk over 3 fresh generators, medians over 15 rounds, the ratio the median of the per-round ratios. Three other worktrees were building throughout: `uptime` before `7:19  up 8 days,  6:18, 1 user, load averages: 24.67 21.37 17.69`, after ` 7:23  up 8 days,  6:22, 1 user, load averages: 28.65 28.66 22.01`. A first run of 9 rounds at the same load put the straddling forest and dry Chunks @ 0.5 m at 106% and 125%; 15 rounds put them at 86% and 78%, so that was load, not the eager Biomes.

```sh
TESSERAE_BENCH_ROUNDS=15 TESSERAE_BENCH_AGAINST=$PWD/bench-gen01 ./bench-gen02 --bench
```

| Case | µs | 01 µs | Ratio |
| --- | --- | --- | --- |
| straddling forest @ 1 m (1 Chunks) | 10333.8 | 12673.2 | 79.3% |
| straddling dry @ 1 m (1 Chunks) | 6895.8 | 9088.8 | 72.5% |
| sky @ 1 m (1 Chunks) | 0.4 | 0.5 | 90.8% |
| deep rock @ 1 m (1 Chunks) | 1170.8 | 1685.9 | 75.5% |
| river @ 1 m (1 Chunks) | 6148.0 | 7149.6 | 78.1% |
| cavern @ 1 m (1 Chunks) | 8364.2 | 9652.8 | 88.8% |
| coarse level 3 @ 1 m (4 Chunks) | 1188.1 | 1559.5 | 85.1% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1760.7 | 2130.7 | 91.3% |
| straddling crop field @ 1 m (1 Chunks) | 31314.5 | 35413.3 | 92.7% |
| crop field column @ 1 m (4 Chunks) | 12118.4 | 13699.7 | 86.4% |
| straddling forest @ 0.5 m (1 Chunks) | 4177.4 | 5143.8 | 86.2% |
| straddling dry @ 0.5 m (1 Chunks) | 3773.4 | 6057.2 | 77.9% |
| sky @ 0.5 m (1 Chunks) | 0.5 | 0.5 | 85.0% |
| deep rock @ 0.5 m (1 Chunks) | 1027.6 | 1554.8 | 65.2% |
| river @ 0.5 m (1 Chunks) | 4013.6 | 5284.2 | 78.4% |
| cavern @ 0.5 m (1 Chunks) | 5978.8 | 7485.2 | 81.1% |
| coarse level 3 @ 0.5 m (4 Chunks) | 1385.2 | 1454.7 | 86.3% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 980.3 | 1250.1 | 83.5% |
| straddling crop field @ 0.5 m (1 Chunks) | 22640.1 | 24225.2 | 92.0% |
| crop field column @ 0.5 m (4 Chunks) | 7285.1 | 8242.8 | 84.4% |

Every case is cheaper, by 7% to 35%; the lone river and the deep rock gain most, the crop-field cases least, as the crop fields ask columns the column cache does not serve.
