# Keep every Chunk under the cap, and leave half of it to the player

Status: ready-for-agent
Blocked by: 01

Spec decision 5.

## What to build

- `MAX_SMOOTHED_PER_CHUNK = MAX_SHAPED_PER_CHUNK / 2`, enforced in the pass in a
  fixed column order, and a `const` assertion that `CHUNK_SIZE^2 <=
  MAX_SHAPED_PER_CHUNK`.
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
