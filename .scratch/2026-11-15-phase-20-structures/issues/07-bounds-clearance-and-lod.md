# 07: Bounds, spawn and coarse levels know about Structures

Status: done

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

The vertical bounds the Server trusts for streaming and open sky widen by the
tallest and deepest Template any Biome lists. Spawn clearance counts Structures,
so a Player never begins inside one. Coarse levels leave Structures out and do
not vote on them.

## Acceptance criteria

- [x] A tower taller than the terrain band and a cellar deeper than the deep-
      rock early-out are generated and lit correctly.
- [x] A World with no Structure keeps its exact bounds and spawn clearance,
      pinned by a test.
- [x] A Player never spawns inside a Structure.
- [x] Coarse levels contain no Structure Voxel.

## Comments

Where things live:

- `Worldgen::surface_bounds_m` widens by `structure_band_m()` (in
  `worldgen/structures.rs`): the terrain band, raised by the tallest Template
  any row lists above its Ground layer and lowered by the deepest below it,
  plus one Voxel each way, the same band `structures_may_reach` already used.
  A World with no Structure pass returns exactly the old bounds (pinned by
  `a_world_without_a_listed_structure_keeps_its_exact_bounds_and_clearance`).
- `Worldgen::chunk` and the Plant pass read a private `ground_bounds_m()`
  (terrain band plus Plant rise), not the widened bounds: Structures stand
  themselves in sky and deep-rock Chunks via `structures_may_reach`, so those
  Chunks keep their early-outs and a World with Structures costs no more
  column sampling in its sky.
- `Worldgen::clear_height_m` counts the top of every Structure whose
  footprint covers a column under the given rectangle
  (`structures_clear_m`); the spawn Eye goes on its roof. The whole bounding
  box counts, Keep and Air cells included: conservative, never inside.
- Coarse levels: `LodGenerators` builds its sources from `params()` alone and
  Templates are not params, so no coarse source has a pass. Pinned by
  `server::lod::tests::structures::coarse_levels_hold_no_structure_voxel`
  (level 0 has thousands of tower Voxels, levels 1 to 3 none, downsampled and
  column-only alike). Coarse `Surface::bounds_m` stays the bare band too.

Lighting:

- The Server's open-sky border (`streaming::above_the_surface`) reads
  `surface_bounds_m().1`, so with the widening a tower Chunk above the
  terrain band is no longer taken for open sky. Core test
  `the_bounds_hold_every_structure_voxel_and_only_air_lies_above_them` pins
  the contract it relies on: every Structure Voxel's floor is inside the
  bounds and the Chunks just above them are uniform air.
- A cellar below the deep-rock early-out needs nothing from the bounds: it is
  lit by the solver from its loaded neighbours like any cave, and nothing
  reads the lower bound to decide darkness (only tests and the coarse
  `Reach`, whose sources have no Structures). The same core test checks a 39 m
  shaft reaches a deep-rock Chunk and is generated there.

For later tickets:

- The lattice's 0.5 m inset keeps every Structure off the columns either
  side of a cell border, and 0 is always a border, so today no Structure
  covers the origin's columns and the first spawn never meets one. The spawn
  guarantee is therefore pinned in core through `clear_height_m`
  (`no_body_clears_below_the_top_of_a_structure_it_stands_over`), not by a
  Server spawn test; if 04 or 14 moves Structures onto the origin it holds.
- `structure_band_m` is sound only while `structure_accepts` puts a
  Structure's Ground layer on a ground Voxel inside the terrain band, as the
  median ground of 04 still does. A rule that lifts or sinks a Structure
  further (levelling in 15) must widen `structure_band_m` by as much.
