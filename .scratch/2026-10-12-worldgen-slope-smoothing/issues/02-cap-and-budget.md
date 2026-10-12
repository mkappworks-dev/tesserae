# Keep every Chunk under the cap, and leave half of it to the player

Status: done
Blocked by: 01

Spec decision 5.

## What to build

- `MAX_SMOOTHED_PER_CHUNK = MAX_SHAPED_PER_CHUNK / 2`, enforced in the pass in a
  fixed column order.
- Replace `generation_places_no_shaped_voxels` with the two tests in the spec.
  The sweep covers 1.0, 0.5 and 0.25 m Voxels and prints the densest Chunk.
- A test of the pass on a synthetic 45-degree surface, where every column is a
  candidate, asserting it stops at the budget and the result is deterministic.
- Update the doc comment on `MAX_SHAPED_PER_CHUNK` and the "worldgen places none"
  sentence in it, and DESIGN.md's "Terrain places none".
- State, with a number from the sweep, how much of the player's half of the cap
  real terrain leaves free.

## Acceptance criteria

- No Chunk in the sweep exceeds the budget; the densest one is quoted.
- The synthetic ramp pins the budget exactly (not "at most" for the saturated
  case).
- A decision on `MAX_SHAPED_PER_CHUNK` itself (change or keep) with the reason
  in Comments.

## Comments

`MAX_SHAPED_PER_CHUNK` is **kept at 1024**; nothing about it changed. What the
sweep measured, at 1 m Voxels with caves off, over 400 Chunks that hold shaped
Voxels (a 20 x 20 square of columns, every layer): median 248, p90 331, max 454
against a budget of 512. So real terrain at the default Voxel size never reaches
the budget (`smoothed_generation_stays_within_the_budget` asserts `<` at 1 m),
and a player keeps at least 570 of the cap in the densest Chunk. At 0.5 m and
0.25 m, with the threshold forced to 0 (the default does not smooth them), a
few Chunks do hit 512, which is what the budget is for.

The saturated case is `a_ramp_that_steps_at_every_column_stops_at_the_budget`,
which asserts the budget runs out part-way along one row and every later row is
cubes. Mesh cost at the budget, from `greedy_mesh/terrain_stairs_at_budget`:
453 us against 341 us for the terrain alone in the same run (+33%), inside the
+50% rule.
