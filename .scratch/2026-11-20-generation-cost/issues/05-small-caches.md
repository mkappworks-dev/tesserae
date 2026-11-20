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
