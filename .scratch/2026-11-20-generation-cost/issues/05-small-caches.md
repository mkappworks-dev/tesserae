# 05: The small caches, each if its symbol is worth it

Status: ready-for-agent

Blocked by: 02, 03, 04

Spec: [Generation cost](../spec.md).

## What to build

First the gate: a sampled `flight` after 04. Each of these is built, one commit each, only if its symbol is more than 3% of generation at some level; otherwise closed in Comments with its figure:
- the cavern field reads a noise corner cache (`fbm3_cached`), as the tunnel fields do;
- `sealed_below_m` and the smoothing's `filled` read the column cache where their points fall on its lattice, and drop their duplicate `water_level_m` call;
- one `claims_near` at the widest margin serves a Chunk, filtered for each use;
- an ore remembers its last vein cell and its draws while a column is walked down.

## Acceptance criteria

- [ ] The gate's figures in Comments.
- [ ] For each built: the fingerprint and the committed digests match, and a test of its own seam (the cached cavern field equals `fbm3`; the seal and `filled` equal their uncached answers; the ore equals today's per Voxel).
- [ ] For each built: bench figures in Comments.

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
