# Mesher pass 2

Status: ready-for-agent
Blocked by: 02

The step's title. Spec decisions 4, 6 and 11.

## What to build

- After pass 1, if the Neighborhood says the Chunk may contain shaped Voxels,
  scan the centre once and for each shaped Voxel append its precomputed quads. A
  quad on the boundary plane is culled when its rectangle is a subset of the
  neighbour's coverage of the opposite face (a cube neighbour culls everything;
  air culls nothing), and otherwise takes the **neighbour's** light. An interior
  quad takes the Voxel's **own** light. One flat `Light` per quad, four copies,
  the same attributes as pass 1.
- UVs are the sub-cell position inside the Voxel over 16, with the same axis
  convention as pass 1, and the same winding rule: counter-clockwise from +d for
  a positive face.
- `ChunkMesh.skipped_shaped` becomes `shaped_voxels`. The unconditional count
  loop at the top of `greedy_mesh` is deleted; terrain, which has no shaped
  entry in its palette, skips pass 2 and that loop both, and the bench should
  show it.
- `MAX_SHAPED_PER_CHUNK` in core. Benchmark pass 2 with a cap's worth of
  scattered stairs on terrain and with a Chunk that is nothing but stairs, then
  set the constant to the largest round number keeping a Chunk at the cap within
  +50% of `mesh_chunk/straddling`, and record both numbers in the constant's doc
  comment.
- Worldgen is **not** changed. A test sweeps generated Chunks and asserts
  `shaped_count() == 0`, as the contract a later generator must update
  deliberately.
- Rewrite the doc on `ATTRIBUTE_MATERIAL`: the spare 16 bits are no longer
  "reserved for step 5"; say what would earn them.

## Tests

- **The hole-free property** (spec decision 4): seeded random scenes in a small
  region of a Chunk - cubes and shaped Voxels in all 24 Orientations, several
  Materials - meshed with `Neighborhood::isolated`; the exact set of exposed
  sub-cell faces computed from a 1/16 occupancy grid of the scene; every emitted
  quad rasterised to sub-cell faces; every exposed face must be covered by a
  quad with the same normal and plane. Hundreds of scenes. A second assertion
  bounds the overdraw: hidden quads exist but a slab floor's interior sides are
  not among them.
- A flight of stairs: quad count equals the sum of the stair's own exposed quads
  minus the culled boundary ones.
- Light: a stair tread carries the stair's own light; its sealed back carries
  the neighbour's; the two are different when the scene makes them different.
- Winding and planarity tests from step 1 extended to pass-2 geometry
  (`triangle_winding_matches_the_declared_normal`,
  `every_quad_is_planar_and_axis_aligned`).
- `declining_never_changes_the_mesh_it_would_have_produced` and
  `light_does_not_revive_either_declined_case` still hold; a uniform shaped
  Chunk meshes and is not an occluder.
- Worldgen emits no shaped Voxel in a sweep of Chunks.
