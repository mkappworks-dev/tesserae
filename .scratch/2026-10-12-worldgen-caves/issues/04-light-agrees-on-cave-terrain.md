# The two light solvers agree on generated cave terrain

Status: done
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

**Tests.** `Lattice` gained a `roof`: Chunks of open sky above the terrain that
are lit from the start and neither reset nor solved, because nothing above them
would light them. Two tests on generated terrain:

- `the_two_solvers_agree_on_generated_terrain_with_a_tunnel_through_it`: the
  first surface Chunk where a tunnel removes a Grass cap, in two columns of two
  Chunks that share a seam. The incremental solver digs the tunnel one Voxel at
  a time into terrain the bulk solver already lit, then refills it in reverse
  (the de-propagation pass on cave geometry); the bulk solver is handed the
  finished terrain. Both comparisons are Voxel for Voxel, and the test asserts
  that some tunnel Voxels are partly lit, so sky really got in and dimmed.
- `the_two_solvers_agree_on_an_enclosed_tunnel_with_lanterns_on_its_seams`: a
  deep Chunk no sky reaches, which `light_chunk` must answer dark. Then a
  lantern on each face a tunnel crosses, on the home side and again on the far
  side, so every seam is carrying light into and out of the Chunk through the
  bulk solver's border.

**Mutations tried**, each against `src/light.rs` alone, run on the tests before
and after. Only a cave scene exposed these (no existing test failed):

- the bulk solver ignores light arriving through `POS_X`, `NEG_Z`, `POS_Z` or
  `NEG_Y`;
- `relight` forgets to regrow from its `NEG_Z`, `POS_Y` or `NEG_X` neighbour.

Ignoring `NEG_X` or `POS_Y` is also caught by older tests, and `darken` never
clearing downwards or `brighten` never spreading are caught by many. The first
version of the lantern test put its lanterns well away from the faces and
caught none of the bulk-border mutations, which is why the lanterns sit on the
seams.

**Read, not assumed.** Every fast path in `light_chunk` is conditioned on the
Chunk or its borders being uniform *and the answer being exact for that case*
(opaque, open sky, nothing to spread), so a Chunk that stopped being uniform
loses speed and nothing else. The Server's `Border::Absent` is dark and
`Border::OpenSky` needs the neighbour's floor above `surface_bounds_m().1`,
which caves cannot move. Nothing relied on "deep rock is uniform" for
correctness.
