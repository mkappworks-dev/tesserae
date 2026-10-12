# The two light solvers agree on generated cave terrain

Status: ready-for-agent
Blocked by: 02

Spec invariant 5. `the_two_solvers_agree_on_the_same_scene` runs on a floor and
a wall. Caves are the first real terrain with overhangs and enclosed pockets.

## What to build

- Extend the agreement test (or add a sibling) so a Chunk from `Worldgen` that
  a cave crosses and has a mouth is solved by `light_chunk` and by `relight` one
  Edit at a time, and the results are compared Voxel for Voxel.
- Cover a Chunk with an enclosed pocket (no mouth), which must come out dark
  without the fast paths hiding a difference.
- Check, by reading, that nothing in `light_chunk` or the Server's solve
  assumed "deep is uniform" for correctness rather than speed.

## Acceptance criteria

- The test fails if `relight` or `light_chunk` is deliberately broken in a way
  only cave geometry exposes (state which change was tried).
- `./scripts/cargo-ci` passes.

## Comments
