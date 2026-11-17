Status: ready-for-agent

# Worldgen performance: Generator version 3 made cheaper

Makes Generator version 3 (DESIGN.md step 17,
[spec](../2026-11-16-phase-17-terrain-and-water/spec.md)) cheaper to generate
without changing what it generates. Starts from `feat/phase-17-terrain-and-water`
at d4179634 and merges after it. Background reading:
[ADR-0030](../../docs/adr/0030-generator-version-3-is-a-layered-field-stack-in-a-declared-band.md),
[ADR-0011](../../docs/adr/0011-coarse-levels-are-derived.md),
[ADR-0003](../../docs/adr/0003-voxel-size-is-per-world.md), and the prior art in
[research.md](./research.md).

## Problem Statement

A flight over a version 3 World costs about 140 ms of level-0 World generation per second of flight on one worker, plus about 90 ms for level 1 (version 2's level 0 costs 191 ms); figures from earlier runs, re-measured in ticket 01. Most of that is not new terrain but the same columns asked again: a sampled flight of the starting commit (`--generator 3`, Terra, 13 s) puts the time here, as shares of the samples inside Chunk generation:

| Where                                                                                   | Level 0 | Levels 1–2 |
| --------------------------------------------------------------------------------------- | ------- | ---------- |
| Plants: each column's Biome asks its altitude, which recomputes the height and drainage | 65%     | –          |
| The seal of rock beside water, which asks nine Water levels per column                  | 6%      | 25%        |
| Ores asking each column's Biome, the same recomputation                                 | –       | 24%        |
| Crop fields and Structure claims                                                        | 13%     | –          |
| Grounds and the cave-scale blend, more Biome lookups                                    | 4%      | 12%        |
| The base height fields                                                                  | 2%      | 5%         |
| The 3D density (overhangs and caverns)                                                  | 2%      | 9%         |

The Plant pass rescans every column within a tree's reach for each of the ten or so Chunks over a Cell, and each scan recomputes the ground under it. The coarse levels' sources are separate generators that solve the same drainage regions and build the same columns level 0 already built.

## Solution

Every Chunk over a Cell, and every LOD level that asks the same column, reads one answer computed once:

- A column cache per Cell (today's per-column-of-Chunks surface cache, renamed) also holds each column's Biome, and every Biome lookup in a Chunk reads it: Plants, grounds, ores and the cave-scale blend.
- The Plants standing on a Cell are decided once and kept, and each Chunk over it only takes the ones whose Voxels fall in it.
- The seal of rock beside water is computed the first time a carve asks for a column, and kept.
- The coarse levels' sources share level 0's terrain and drainage, and the level-1 source, which differs from level 0 only by its surface features, shares its column cache.

Then, only if the numbers still call for it after the caches: SIMD noise, bit for bit the scalar result, and a coarse lattice for the cavern and tunnel fields. Each is gated on a re-measure and closed with its figure if it does not pass.

## User Stories

1. As a Player flying over a version 3 World, I see the ground fill in ahead of me sooner, because the workers spend less time generating each Chunk, so that the richer terrain does not cost me streaming.
2. As a Player, I see exactly the terrain, water, caves, Plants and Structures the starting commit generates, so that this work changes the cost and nothing I can see.
3. As a Player with a version 1 or 2 World, my World is exactly as it was, Voxel for Voxel.
4. As a Player at 0.5 m or 0.25 m Voxels, generation is cheaper too, and the same columns stand in the same metres as at 1 m.
5. As a Player looking at distant land, the coarse levels fill in sooner, because they stop solving drainage and building columns level 0 already did.
6. As a Player on a machine with less memory, the generator holds no more memory than it did, so that the caches do not crowd out Chunks and meshes.
7. As a Player standing in a forest, each tree is still decided alike from every Chunk that reaches it, so that no tree is cut at a Chunk border.
8. As a Player beside a lake, no cave opens onto the water, exactly as before.
9. As a Player under the water table, a flooded cavern stays sealed from dry tunnels, exactly as before.
10. As a Player in rugged land, overhangs never reach down to water or over a Structure, exactly as before.
11. As the Server, a generated Chunk is the same in any Chunk order, with a cold or a warm cache, and on any worker, so that generation stays a pure function of the seed and metres.
12. As the Server, a cache never becomes an input: dropping it and generating again gives the same Chunk.
13. As the Server, the caches are bounded, and reaching a bound drops the oldest half, not all of it, so that a flight does not throw away the columns around the Eye.
14. As the Server, two workers asking for the same column at once never block on one global lock for every lookup.
15. As the Server, two workers that both compute a column's seal or Plants agree, so that a race costs time and never changes a Voxel.
16. As the Server, the direct column generators for the farthest levels stay as they are.
17. As a Game author, nothing I declare changes: no new tuning, no new field in the Declaration or the save.
18. As a Mod author, nothing crosses the WIT contract that did not before.
19. As a contributor, I can run one bench that times version 3's Chunks as interleaved CPU time against a build of the starting commit, so that host load does not decide the result.
20. As a contributor, the bench includes a column of Chunks under a Biome that stands trees, because Plants are the largest cost and the default fixture has none.
21. As a contributor, the bench names its Chunks by a fixed scan, so that a figure means the same Chunk on every run.
22. As a contributor, each ticket records its bench figures before and after, and the flight profile is recorded at the start, after the Plants ticket and at the end.
23. As a contributor, each ticket proves version 3 is bit-identical to the starting commit, so that "faster" never hides "different".
24. As a contributor, a SIMD noise path gives the scalar result bit for bit on every CI architecture, and a test proves it.
25. As a contributor, an optimisation the measurement no longer justifies is closed with its figure rather than built anyway.
26. As a reviewer, each optimisation is one commit with its tests and its figures.
27. As the author of ticket 14 (on hold), I find version 3 not yet frozen, its digests not yet recorded and `GENERATOR_VERSION` unchanged.
28. As a contributor reading the code, the per-Cell cache is not called a Stack, which in this project's language is an Item and a count.

## Implementation Decisions

**Order.** By the measured share: the column cache and its Biome (with the lock and eviction changes), Plants once per Cell, the lazy seal, sharing with the coarse sources, then the two gated tickets. Each is one commit.

**Identity.** Every ticket before the gated ones leaves version 3 bit-identical to the starting commit at 1.0 and 0.5 m, and versions 1 and 2 keep their recorded digests (`generator_version.rs`, `surface_features.rs`, `settlement_worldgen.rs`, `coarse_generation.rs` and the others the step 17 spec lists). Version 3 gets no committed digest here; that is ticket 14's, and `GENERATOR_VERSION` does not move.

**The column cache.** The per-Cell cache of surface heights, Water levels, overhang reach and grounds is renamed from `Stack`, which the glossary reserves for an Item and a count. It gains each column's Biome (an id per column, about 1 KB a Cell); the Plant, ore, ground and cave-scale lookups read it instead of asking `biome_at`, which recomputes the column's height and drainage. A lookup outside the Chunk's own Cell builds the neighbouring Cell's cache. The map becomes a read-write lock with a thread-local record of the last Cell asked, as the drainage regions already have. At its bound of 1024 Cells it drops the oldest half.

**Plants once per Cell.** The Plants standing on a Cell's columns are decided once, with the crop fields and footprints that decide them, and kept in a separate bounded map of compact entries (the feature, the base, the draw), about 7 KB a Cell. A Chunk takes the Plants from its own and neighbouring Cells within a tree's reach and filters them to its height. Only a generator that stands Plants fills the map, so a coarse source sharing the column cache adds nothing to it.

**The seal.** The height below which rock beside water stays solid is computed the first time a carve asks for a column and kept in the column cache, one atomic per column (about 4 KB a Cell). It is a pure function, so two workers that race write the same value.

**Sharing with the coarse levels.** A generator can be made to share another's work, if its seed and every terrain parameter match: it always adopts the other's terrain, so one drainage solve serves every level, and adopts the column cache only at the same Voxel size. The coarse levels' sources use it from level 0's generator; the level-1 source, level 0 without surface features, shares the column cache. The direct column generators, which ask about a Voxel's middle with Sea level shifted by half a Voxel, are left alone.

**Memory.** The new caches add up to about 12 MB at their bounds; sharing removes up to six drainage caches (each up to about 11 MB) and one column cache (up to about 25 MB). The net is lower than today.

**SIMD, gated.** Runs only if noise is still more than 15% of generation once the caches land. Uses the `wide` crate (stable, one code path, an exact floor on every backend; its fused multiply-add is never used), batched four wide where a caller already walks a row: building the column cache, the drainage window and the overhang push down a column. The output is the scalar output bit for bit, octave sums kept per lane. Otherwise closed as `wontfix` with the figure.

**Coarse lattice, gated.** Runs only if the 3D density is still more than 10% of generation after SIMD. Covers the cavern field and the two tunnel fields only, on a 4 m lattice anchored at the World origin in metres, so it holds at every Voxel size; each field is interpolated before it is thresholded, and the interval bounds widen by a few ulps. The overhang push stays exact per Voxel. Cave shapes may move, which version 3 allows only because ticket 14 has not frozen it; the water rules in `terrain_v3.rs` must still hold exactly. Otherwise closed as `wontfix` with the figure.

## Testing Decisions

- The seam is `Worldgen::chunk`, the one the version 3 property tests already use. A good test asks what a Chunk holds, never what a cache holds.
- Cache independence: the existing any-order and cold-cache test, extended to Plants, a coarse source sharing level 0's work, and every Biome lookup the cache serves; a Chunk generated after dropping every cache equals one generated warm.
- Bit-identity to the starting commit: a fingerprint of a fixed grid of Chunks at 1.0 and 0.5 m (with caves, water, Biomes, Plants and Structures), compared before and after each ticket and kept out of the commit, since a committed version 3 digest is ticket 14's.
- Versions 1 and 2: the recorded digest tests pass unchanged.
- The water rules (`terrain_v3.rs`): water never beside or above air, flooded caverns sealed from tunnels, no overhang near water or Structures, at every Voxel size.
- Sharing: a coarse source made to share level 0's work generates the same Chunks as one made alone (prior art: the Server's LOD tests comparing a source with a fresh one).
- SIMD: a property test over many seeds and positions asserting the batched result equals the scalar one bit for bit, run on every CI architecture.
- Benchmarks: a new bench in the style of `benches/structures.rs` (interleaved rounds, thread CPU time, medians) over a synthetic World shaped like Terra (a forest Biome with a clumped tree, a dry Biome with its own ground and cave scale, the fixture's ores, water and caves): straddling, both early-outs, a river, a cavern, a coarse level-3 Chunk and a column of Chunks with Plants, found by a fixed scan. A/B against a binary of the starting commit after every commit, under the host-load rules: `uptime` checked and recorded, no other build running.
- The flight profile (`scripts/profile-frames --generator 3`, three interleaved rounds) at the start, after the Plants ticket and at the end, per level and stage.

## Out of Scope

- Freezing version 3, recording its digests and moving `GENERATOR_VERSION` (ticket 14), and its docs (ticket 15).
- Any change to what version 3 generates, except cave shapes under the gated lattice ticket.
- Changes to the direct column generators of the farthest levels.
- Meshing, light and rendering costs.
- New tuning, Declaration fields or save fields.
- A coarse lattice for the overhang density.

## Further Notes

- Research, with sources: [research.md](./research.md). Production engines sample density every 4×8×4 and interpolate; the 10 m overhang field is too fine for that, the cavern field is not, and the tunnel fields survive at 4 m cells. `std::simd` is still nightly-only; Rust never fuses `a*b + c`, so lane-wise SIMD keeps the scalar bits.
- The first step of every ticket is a failing test, per `/tdd`.
- Tickets are in [issues/](./issues/).
