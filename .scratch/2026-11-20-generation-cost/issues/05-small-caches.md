# 05: The small caches, each if its symbol is worth it

Status: done

Blocked by: 02, 03, 04

Spec: [Generation cost](../spec.md).

## What to build

First the gate: a sampled `flight` after 04. Each of these is built, one commit each, only if its symbol is more than 3% of generation at some level; otherwise closed in Comments with its figure:
- the cavern field reads a noise corner cache (`fbm3_cached`), as the tunnel fields do;
- `sealed_below_m` and the smoothing's `filled` read the column cache where their points fall on its lattice, and drop their duplicate `water_level_m` call;
- one `claims_near` at the widest margin serves a Chunk, filtered for each use;
- an ore remembers its last vein cell and its draws while a column is walked down.

## Acceptance criteria

- [x] The gate's figures in Comments.
- [x] For each built: the fingerprint and the committed digests match, and a test of its own seam (the cached cavern field equals `fbm3`; the seal and `filled` equal their uncached answers; the ore equals today's per Voxel).
- [x] For each built: bench figures in Comments.

## Comments

### The gate, 2026-11-20

From the mid-point sampled flight in [ticket 04](./04-crop-fields-once.md#the-mid-point-flight-2026-11-20), every one of the four clears 3% at some level, so all four are built:

| Cache | Symbol | Level 0 | Levels 1–5 |
| --- | --- | --- | --- |
| Ores remember their vein cell | `OreField::at` | 12.0% | 22.6% |
| The cavern field's corner cache | `Worldgen::cavern_open` | 15.9% | 12.4% |
| One `claims_near` a Chunk | `Worldgen::claims_near` | 17.3% | 0.0% |
| The seal and `filled` read the column cache | `Worldgen::sealed_below_m`; `Worldgen::ground_at` (`filled`) | 1.9%; 7.7% | 9.1%; 2.0% |

### Ores and the cavern field

- Ores remember their vein cell: `OreField::at` takes an `OreCells`, which holds, for each ore, the cell it last asked about and the vein drawn there (or none). A Chunk's column loop and `sow_deep_rock` each keep one for the Chunk. Keyed by the whole cell, so any walk order answers the same. `an_ore_walking_a_column_answers_as_it_does_per_voxel` walks columns at every Voxel size from 0.25 to 32 m over 96 ore tables and compares every Voxel with a cache that has seen nothing. The fingerprint and the committed digests match.
- The cavern field reads a noise corner cache: `Caverns::carves_near` and `Caverns::walls_in` ask `fbm3_cached`, and a Chunk keeps one `Fbm3Cache` for each place its column loop asks the field (`cavern_open`, `walled_in`, the fill's `hollow_at` below, the spring's `hollow_at` beside, and the floor check above). Shafts and `carved` keep the plain field. `the_cached_cavern_field_is_the_plain_one_bit_for_bit` walks one cache down columns at 0.5 to 32 m Voxels over 48 seeds, also asking a neighbour each Voxel as a spring does, and compares every value with `fbm3`'s bits. The fingerprint and the committed digests match.

### The seal, `filled` and the claims, 2026-11-20

- **The seal and `filled` read the column cache.** `sealed_below_m` and `filled` read a lattice point's contained surface and water level, and `filled` its own column's overhang amplitude, from the Chunk's own cache, its one-column border included, or from the cache of the point's Cell where one is already held. Neither ever builds a cache: a Cell's is over a thousand columns, and `filled` asks five from a Chunk whose neighbour may not be made yet. An unheld or off-lattice point is asked as before. The seal's duplicate `water_level_m` call is gone. `filled` asked from a Plant's `eased` has no Chunk, so it reads only held caches.
- **One `claims_near` a Chunk.** The Chunk's full pass asks `claims_near` once, at the widest margin any pass reads (twice the scatter margin, the cave clearance, or the overhang clearance plus the column past the Chunk). `claims_within` filters it to each pass's own region: the cave clearance for `cave_free`, twice the scatter margin for the Structures stood and what scatter and crop fields keep off, and the overhang clearance plus one for what the overhang density keeps off, which the voxel loop and `filled` both read. A Settlement laid out now keeps its founding's reach, so the filter can apply `laid_settlements_near`'s two tests. The sky and deep-rock early-outs, `carved`, `ground_top` and a Cell's Plants keep their own asks, since none of them has the Chunk's ask to read.

Proof: `the_seal_and_filled_read_held_caches_as_asking_answers` holds the seal and `filled`, read from a Chunk's cache and a held neighbour's, equal to a World that holds no cache and asks every column. It runs at 1 m, 0.5 m and 0.75 m, on three banks at seed 3907 where the overhang density works beside water. `a_chunks_claims_filtered_are_each_narrower_ask` holds the filtered claims equal to each narrow `claims_near` at every margin a pass reads, over 144 Chunk columns of the content fixture at 1 m and 0.5 m, Settlements and Structures among them. It also holds the overhang keep-off, read a column past the Chunk, covering each column as the column's own Chunk's `overhang_kept_off` does. Dropping the Settlement filter's trail reach fails the first; widening the keep-off one column short fails the second. The uncommitted fingerprint matches all 58 baseline lines after each commit, the `offlattice` lines included. `generator_version` and the tesserae-core tests pass. The figures are the parent's.

### Bench figures, 2026-11-20

Each cache's commit against the one before it, so a column is that cache alone: ores (48a357d0) against 5390bc3a's generator, the cavern field (66b893f4) against ores, the seal and `filled` (42945a89) against the cavern field, the claims (06f09582) against the seal. Each commit was exported with `git archive`, built with `RUSTFLAGS="-D warnings" cargo bench --locked --no-run -p tesserae-core --bench worldgen_v3` in its own target dir and copied out; interleaved rounds, thread CPU time per Chunk over 3 fresh generators, medians over 15 rounds, each ratio the median of the per-round ratios. Nothing else built; `uptime` from `9:45  up 8 days,  8:43, 1 user, load averages: 10.73 13.88 12.08` before the first to ` 9:49  up 8 days,  8:48, 1 user, load averages: 1.73 7.13 9.52` after the last.

```sh
TESSERAE_BENCH_ROUNDS=15 TESSERAE_BENCH_AGAINST=$PWD/bench-<previous> ./bench-<commit> --bench
```

| Case | Ores | Cavern field | Seal and `filled` | Claims |
| --- | --- | --- | --- | --- |
| straddling forest @ 1 m (1 Chunks) | 99.8% | 99.7% | 69.4% | 100.2% |
| straddling dry @ 1 m (1 Chunks) | 98.3% | 99.8% | 76.0% | 100.2% |
| sky @ 1 m (1 Chunks) | 100.2% | 93.9% | 99.8% | 99.8% |
| deep rock @ 1 m (1 Chunks) | 76.2% | 100.0% | 99.8% | 100.6% |
| river @ 1 m (1 Chunks) | 99.3% | 100.3% | 87.5% | 100.5% |
| cavern @ 1 m (1 Chunks) | 96.9% | 98.5% | 77.3% | 99.9% |
| coarse level 3 @ 1 m (4 Chunks) | 91.4% | 99.5% | 79.5% | 97.4% |
| forest 3x3 columns @ 1 m (36 Chunks) | 96.2% | 91.5% | 67.5% | 100.0% |
| straddling crop field @ 1 m (1 Chunks) | 100.0% | 100.2% | 83.3% | 100.2% |
| crop field column @ 1 m (4 Chunks) | 97.5% | 98.7% | 84.0% | 100.1% |
| straddling forest @ 0.5 m (1 Chunks) | 99.4% | 99.6% | 98.7% | 100.3% |
| straddling dry @ 0.5 m (1 Chunks) | 99.3% | 100.0% | 93.9% | 100.5% |
| sky @ 0.5 m (1 Chunks) | 107.9% | 88.0% | 93.1% | 106.8% |
| deep rock @ 0.5 m (1 Chunks) | 80.4% | 99.2% | 99.9% | 100.3% |
| river @ 0.5 m (1 Chunks) | 99.2% | 99.9% | 99.9% | 100.3% |
| cavern @ 0.5 m (1 Chunks) | 96.6% | 98.5% | 92.2% | 100.0% |
| coarse level 3 @ 0.5 m (4 Chunks) | 88.3% | 95.1% | 73.3% | 98.1% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 93.0% | 95.7% | 83.5% | 100.2% |
| straddling crop field @ 0.5 m (1 Chunks) | 98.9% | 100.5% | 99.7% | 100.3% |
| crop field column @ 0.5 m (4 Chunks) | 98.6% | 97.9% | 94.0% | 100.1% |

The seal and `filled` move most: a forest column of Chunks @ 1 m costs 68% of what it did, the straddling forest Chunk 69%, coarse level 3 73% to 80%. The ores take a fifth off deep rock and a tenth off coarse level 3; the cavern field a few percent off the forest columns. The claims do not move the bench, whose World has one hut row; Terra has many Structures and Settlements, and the sampled flight measures them.

### The flight after 05, 2026-11-20

| Fixture | Level | Stage | 4e78a50f | Mid-point (fded14c8) | After 05 (fa6fd574) | After / start |
| --- | --- | --- | --- | --- | --- | --- |
| `flight` | 0 | generate | 184.6 | 58.1 | 46.6 | 25% |
| `flight` | 0 | relight | 91.3 | 102.4 | 102.0 | 112% |
| `flight` | 0 | mesh | 27.5 | 27.6 | 30.7 | 112% |
| `flight` | 1 | generate | 251.4 | 220.9 | 199.8 | 79% |
| `flight` | 1 | mesh | 274.9 | 277.6 | 302.6 | 110% |
| `flight` | 2 | generate | 49.1 | 34.1 | 29.5 | 60% |
| `flight` | 3 | generate | 8.5 | 4.6 | 4.5 | 52% |
| `flight` | 0–3 | generate | 495.0 | 317.3 | 281.7 | 57% |
| `sprint-flight` | 0 | generate | 568.4 | 247.0 | 202.2 | 36% |
| `sprint-flight` | 0 | relight | 405.6 | 402.4 | 449.0 | 111% |
| `sprint-flight` | 0 | mesh | 38.9 | 39.7 | 44.4 | 114% |
| `sprint-flight` | 1 | generate | 797.4 | 612.6 | 537.2 | 67% |
| `sprint-flight` | 1 | mesh | 151.9 | 184.4 | 199.2 | 131% |
| `sprint-flight` | 2 | generate | 258.6 | 165.2 | 151.1 | 58% |
| `sprint-flight` | 3 | generate | 18.9 | 9.5 | 9.4 | 50% |
| `sprint-flight` | 0–3 | generate | 1640.2 | 1028.3 | 899.6 | 55% |
| `teleport` | 0 | generate | 469.5 | 371.3 | 337.5 | 72% |
| `teleport` | 0 | relight | 89.6 | 89.9 | 98.0 | 109% |
| `teleport` | 0 | mesh | 34.8 | 33.2 | 37.5 | 108% |
| `teleport` | 1 | generate | 493.6 | 370.5 | 345.2 | 70% |
| `teleport` | 1 | mesh | 132.7 | 137.6 | 146.2 | 110% |
| `teleport` | 2 | generate | 428.2 | 266.3 | 255.7 | 60% |
| `teleport` | 3 | generate | 63.8 | 30.0 | 33.6 | 53% |
| `teleport` | 0–3 | generate | 1453.4 | 1045.6 | 970.7 | 67% |

The teleport's level 0 settles in 2.05 s (1.89 to 2.14) after 05, 2.11 s at the mid-point and 2.53 s (2.49 to 2.65) at the start.

How they were taken: builds of 4e78a50f, fded14c8 (the mid-point) and fa6fd574 (this spec's tickets 01 to 05 and the sister spec's 01, 02 and 04), each `RUSTFLAGS="-D warnings" cargo build --locked -p tesserae --features profiling --release`, copied out; one `scripts/profile-frames --fixtures flight sprint-flight teleport --repeats 3` series per build per round, nothing else building or running a `tesserae` process. Three rounds of 4e78a50f and fded14c8 (7:55 to 10:10) and two of fa6fd574 (10:10 and 10:41), interleaved with them; medians over the logging-on runs, worker CPU in ms per second of the fixture. `uptime` load from 3.86 to 7.03 throughout.

In the sampled flight of fa6fd574 (six processes, as in ticket 04), level 0's generation is 1849 samples against 2416 at the mid-point, the coarser levels 4333 against 5804. One `claims_near` a Chunk did not move it: `claims_near` is 21.9% of level 0 against 17.3% of a larger total, about 400 samples either way. Its cost is the Settlement layout under it (`Worldgen::crossing` 11.6%, `lay_out` 6.5%, `plot` 5.6% of level 0), which each call lays out again; that is outside this spec.
