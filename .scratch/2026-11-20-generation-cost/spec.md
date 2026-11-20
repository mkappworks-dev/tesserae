Status: ready-for-agent

# Generator version 3, cheaper again: drainage, cave Biomes, crop fields

Follows [Worldgen performance](../2026-11-17-worldgen-performance/spec.md) on the same branch, which made Generator version 3's level 0 81% cheaper in a flight and left its cost in places it did not touch. Version 3 stays frozen: its output is bit-identical at every commit and `GENERATOR_VERSION` does not move. Its sister spec, [Terrain fill latency](../2026-11-20-terrain-fill-latency/spec.md), works on the same branch on how soon generated terrain is drawn; both are judged by the same flight runs. Background reading: [ADR-0030](../../docs/adr/0030-generator-version-3-is-a-layered-field-stack-in-a-declared-band.md), [ADR-0011](../../docs/adr/0011-coarse-levels-are-derived.md).

## Problem Statement

A Player flying over Terra waits for the ground near the Eye, and at the in-game top speed of 64 m/s generation is most of what the three workers do: per second of a `sprint-flight`, level 0's generation takes 583 ms of worker CPU, LOD level 1's 814 ms and level 2's 263 ms, of about 2.3 s in all. A teleport, which refills the view as a spawn does, takes 2.55 s for level 0 to settle.

In a sampled 12 m/s flight, level 0's generation is mostly not terrain, and the coarser LOD levels spend much of theirs hashing:

| Symbol, inclusive                                          | Level 0 | Levels 1–5 |
| ---------------------------------------------------------- | ------- | ---------- |
| `fields_near` and `field_accepts` (crop fields)            | 70%     | –          |
| `surface_height_m`, uncached (`contained_at_m`)            | 47%     | 2%         |
| `Drainage::wet`                                            | 45%     | 12%        |
| `Base::base_height_m`                                      | 14%     | 5%         |
| `OreField::at`                                             | 4%      | 15%        |
| `noise::fbm3_with`                                         | 4%      | 15%        |
| `RandomState::hash_one` (`[i32; 3]` keys)                  | 3%      | 12%        |

- Every Chunk decides the crop fields within a tree's reach of it again, the ten or so Chunks over a Cell alike, and `cell_plants` and `plants_clear_m` decide them once more; each held Lattice cell tries up to 36 places, each asking the ground, Water and Biome of columns the column cache does not serve.
- Building a column cache asks `Drainage::wet` twice for each column, once for the valley cut and once for the Water level, though the level never reads the ground; asking each column's Biome then recomputes its base height and asks `wet` a third time.
- The cave Biomes of a Chunk are kept in a SipHash map keyed by `[i32; 3]` that every rock Voxel deeper than 12 m reads.
- The cavern field, the seal of rock beside water, the smoothing's neighbour test, the Structure claims and the ores each recompute per Voxel or per Chunk what a neighbour Voxel or Chunk already asked.
- `Drainage::wet` scans 64 lattice nodes per point for stream segments, though most points have none in reach.
- A sky Chunk between a column's top and the band's top still runs the whole pass.
- A lone Chunk from a cold generator builds whole column caches for every neighbouring Cell a tree's reach and crowding touch: river @ 0.5 m takes 4.9 ms once the bench stops counting a drainage region solve, three quarters of it in those caches, against the 1.1 ms the last spec's ticket 01 recorded under its narrower warm-up. Ticket 01 times both builds under the same warm-up before any ratio is claimed.

## Solution

Each cost is answered once, and every Chunk and LOD level that asks again reads it, keeping every Voxel bit for bit:

- One `Drainage::wet` per raw column answers both its cut and its Water level, and the column's Biome is decided from the values the column cache already computed.
- A Chunk's cave Biomes are kept in a dense array, or not kept at all where a Voxel is wider than a cave Biome cell.
- Each crop-field Lattice cell's field is decided once and kept in a bounded map.
- Then, each gated on a re-measure: the cavern field reads a noise corner cache; the seal and the smoothing's neighbour test read the column cache; one Structure claim lookup serves a Chunk; an ore remembers its vein cell down a column; `Drainage::wet` walks a per-region list of the stream segments in reach; a Chunk above everything that can stand over its columns returns air; and a lone Chunk decides its neighbours' Plants only on the strip it needs.

## User Stories

1. As a Player flying over Terra, the ground, trees and Structures near me fill in sooner, because the workers spend less time generating each Chunk.
2. As a Player, I see exactly the terrain, water, caves, cave Biomes, ores, Plants, crop fields and Structures the starting commit generates, Voxel for Voxel, in an existing World and a new one.
3. As a Player at 0.5 m Voxels, the same holds, and the same fields stand in the same metres.
4. As a Player beside a farm, a crop field is whole across Chunk borders, decided alike from every Chunk that reaches it, warm or cold, in any order.
5. As a Player looking at distant land, the coarser LOD levels fill in sooner, because they stop hashing and asking the drainage again.
6. As the Server, a generated Chunk is the same in any Chunk order, with any cache cold or warm, on any worker; dropping every cache and generating again gives the same Chunk.
7. As the Server, the new caches are bounded, drop the oldest half at their bound, and never block every lookup on one lock; two workers that race to decide the same field cell store the same answer.
8. As a Game author and a Mod author, nothing I declare or call changes.
9. As a contributor, the bench has a World with crop fields like Terra's, near water and anywhere, and cases over them, because the fields are the largest cost and the bench has none.
10. As a contributor, the bench's warm-up solves every drainage region a case's Chunks reach, so a lone Chunk's figure is generation and not a region solve.
11. As a contributor, each ticket records its bench figures against the baseline binary, and the flight profile and a sampled flight are recorded at the start, after the crop-field ticket and at the end.
12. As a contributor, each ticket proves version 3 bit-identical with the committed digests and the uncommitted fingerprint.
13. As a contributor, an optimisation the measure no longer justifies is closed with its figure.
14. As a reviewer, each optimisation is one commit with its tests and figures.

## Implementation Decisions

**Order.** The bench and baseline, then the cheap verified wins (one `wet` a column, the dense cave Biomes), then the crop-field cache, then the gated tickets by their measured share. Interleaved on the branch with the sister spec as: this spec's 01 to 03, the sister's 01 to 03, this spec's 04, then the rest of both by measured share. Each ticket is one commit.

**Identity.** Every commit leaves version 3 bit-identical: the committed version 3 digests pass unchanged, `GENERATOR_VERSION` does not move, and the uncommitted fingerprint (`crates/tesserae-core/tests/v3_fingerprint.rs`, in `.git/info/exclude`) matches its baseline. The fingerprint gains a grid over the crop-field World, its baseline taken at the bench commit.

**One drainage ask a column.** `Drainage::wet`'s Water level does not read the ground height, so one call at the base height gives both the cut and the level of a raw column. `build_columns` and `contained_at_m` make that one call. Each column's Biome is decided while its cache is built, whenever the generator has Biomes, from the fields, base height and cut the raw pass computed, rather than lazily by `site` asking them again; the sea-floor rock and overhang reach read them too. The cache's byte per column already holds the Biome, so no memory is added; in Terra every Cell with a Chunk on the ground asks all its columns' Biomes for its grounds and Plants anyway, so only a Cell nothing asks pays the climate noise for nothing. The four-wide path stays bit for bit the scalar one, and a test proves it over many seeds and points.

**The cave Biomes.** A Chunk keeps its cave Biome cells in a dense array indexed from its origin cell, sized by how many cells a Chunk spans at its Voxel size; where a Voxel is at least a cave Biome cell wide, nothing is kept, since no two Voxels share a cell.

**The crop-field cache.** A Lattice cell's field is a pure function of the seed, the Biome, the entry, the cell and the Voxel size: whether the cell is held, and the first of its drawn places the ground takes. It is kept per `(Biome, entry, cell)` as the placed field or none, in a bounded map like `CellMap`. `fields_near` reads it for every Lattice cell its region touches, the "a later entry gives way to an earlier one it crosses" test included. `field_accepts` is unchanged; it is only asked once.

**The small caches, gated together.** After the crop-field cache, a sampled flight decides each of these: the cavern field reading a corner cache as the tunnel fields already do (`fbm3_cached`); the seal of rock beside water and the smoothing's `filled` test reading the column cache where their points fall on its lattice, and their duplicate `water_level_m` call dropped; one `claims_near` at the widest margin per Chunk, filtered for each use; an ore remembering its last vein cell and its draws while a column is walked down. Each is built only if its symbol is more than 3% of generation at some level, one commit each; otherwise closed with its figure.

**Drainage, gated.** Runs if `Drainage::wet` is still more than 15% of level-0 generation after the cheaper wins. A region gains, per lattice cell, the stream segments its stencil reaches, in the order the scan meets them; `wet` walks that list and skips the meander noise where it is empty. The maxima over the same segments in the same order keep the result bit for bit.

**The sky early-out, gated.** Runs if Chunks above their columns' ground are more than 10% of level-0 generation. A Chunk whose floor is above `surface_top_m` over its footprint widened by a tree's reach (the sampled, cached bound relight already trusts for open sky, which holds the overhang, the Water level and the room above the ground), with no Structure claim near it reaching its floor, holds only air and returns air without the column pass, the Plant pass or `fields_near`. Taking each neighbour Cell's column tops instead would build their caches, the lone Chunk's cost. `structures_may_reach` checks only the World's band, so the claims near the Chunk are asked. A sweep test proves every Chunk it takes is air in the full pass.

**The lone Chunk, gated.** Runs if, in the flight, building caches for Cells no level-0 Chunk stands over is more than 10% of level-0 generation. A neighbour Cell not yet held then has its Plants decided only on the strip within a tree's reach and crowding of the Chunk, from Biomes asked point by point, as `surface_height_m` already answers an unheld Cell; a held Cell keeps the cached path. Otherwise closed as `wontfix`: in a flight, a spawn and a teleport, the next Chunks of the same ring ask for those Cells within milliseconds, so the work is moved, not added.

## Testing Decisions

- The seam is `Worldgen::chunk`: a test asks what a Chunk holds, never what a cache holds.
- Cache independence: a field crossing a Chunk border is the same from both Chunks, cold or warm, in any order; a Chunk generated after `forget_caches` equals one generated warm, over the crop-field World.
- One drainage ask: the fused raw column and Biome equal the separate asks bit for bit, over many seeds and points, four-wide and scalar.
- Drainage, if built: `wet` with the segment list equals the scan bit for bit, including points with no segment in reach and beside a region border.
- The early-out, if built: over a sweep of columns at 1.0 and 0.5 m, every Chunk it takes is air in the full pass, over trees, fields, Water and Structures.
- Bit-identity: the committed digests and the uncommitted fingerprint, at every commit.
- Benchmarks: the version 3 bench, interleaved CPU time against the bench commit's binary, after every commit, with `uptime` recorded and nothing else building. The flight profile (`scripts/profile-frames --fixtures flight sprint-flight teleport`, three interleaved rounds) and a sampled `flight` at the start, after the crop-field ticket and at the end.

## Out of Scope

- Any change to what version 3 generates, any new Generator version, or moving `GENERATOR_VERSION`.
- How soon generated terrain is drawn: streaming, light, meshing and the loading screen are the sister spec's.
- LOD level 2 drawn from direct columns as level 3 is, which would drop caves from it: a visual change, kept as an option should the sister spec's 64 m/s gate fail on generation.
- Changes that would move Voxels: a per-column cave Biome lattice, a five-point seal, one hash shared by all ores, the Biome asked without the river cut. Each needs a new Generator version.
- New tuning, Declaration fields or save fields.

## Further Notes

- The starting flight figures, 2026-11-20, at 4e78a50f (`tesserae-head`, `RUSTFLAGS="-D warnings" cargo build --locked -p tesserae --features profiling --release`), three runs each of `flight`, `sprint-flight` and `teleport`, all passing the runner's audit, `uptime` load from 3.80 to 5.88. Worker CPU, ms per second, medians:

  | Fixture         | L0 generate | L0 relight | L0 mesh | L1 generate | L1 mesh | L2 generate | L3 generate |
  | --------------- | ----------- | ---------- | ------- | ----------- | ------- | ----------- | ----------- |
  | `flight`        | 176         | 90         | 26      | 247         | 272     | 48          | 9           |
  | `sprint-flight` | 583         | 404        | 39      | 814         | 155     | 263         | 20          |
  | `teleport`      | 456         | 89         | 34      | 499         | 130     | 428         | 63          |

  The teleport's level 0 settles 2.55 to 2.58 s after the jump, and the coarser levels are generated by 7.5 to 7.8 s.

- The lone Chunk, 2026-11-20, at 4e78a50f, from an uncommitted harness timing each bench case with the bench's warm-up and with every region within four Cells solved first (medians of 9 fresh generators, µs a Chunk): river @ 0.5 m 13,374 and 4,890; cavern @ 0.5 m 16,620 and 5,255; river @ 1 m 15,354 and 5,185; cavern @ 1 m 16,788 and 8,102; the bench case `coarse level 3` @ 0.5 m 3,726 and 1,252; the straddling, deep-rock and forest cases move within noise. With regions solved, 74% of river @ 0.5 m is building neighbour Cells' column caches under `cell_plants`. The river and cavern Chunks sit beside the origin, where four 2 km drainage regions meet, and a neighbour Cell's cache border crosses into a region the bench's warm-up never solved.
- The first step of every ticket is a failing test, per `/tdd`.
- Tickets are in [issues/](./issues/).
