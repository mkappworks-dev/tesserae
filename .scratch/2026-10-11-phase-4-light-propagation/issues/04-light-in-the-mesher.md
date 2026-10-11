# Light in the mesher

Status: ready-for-agent
Blocked by: 01

Light becomes part of the greedy merge key and a vertex attribute on the
`ChunkMesh`. Spec decisions 3 and 4.

## What to build

In `mesh.rs`:

- `LitChunk<'a> { chunk: &'a Chunk, light: &'a ChunkLight }`, so
  `mesh_chunk(center: LitChunk, neighbors: [Option<LitChunk>; 6])` and
  `Neighborhood::new` keep two arguments instead of four. Four would also trip
  `clippy::too_many_arguments`, which CI fails on.
- `Neighborhood` grows a parallel `Box<[Light]>` shell, written in the same
  index order as the Voxel shell and read only directly across a face, exactly
  like the Voxel shell.
- `ChunkMesh.light: Vec<u32>`, one per vertex, carrying `Light`'s packed byte.
- The greedy mask key becomes `(MaterialId, Light)`. The light is the light of
  the Voxel **in front of** the face, which the mask pass already fetches to
  test occlusion - take it from there rather than adding a second read.

## The fast paths, which must be checked and not argued

`emits_no_faces` is unchanged: light can only split a quad that would otherwise
merge, never create or remove a face, because face visibility is a question
about geometry alone. Add a test that gives both fast-path fixtures
deliberately non-uniform light and asserts the mesh is still empty.

## Benches

Update `benches/meshing.rs` for the new signatures and add a case that actually
exercises the split: a Chunk with a light gradient across its top face, so the
merge-key cost is measured and not merely assumed. Record before/after for
`greedy_mesh/*`, `mesh_chunk/*` and `neighborhood_isolated` against the
step-3 baselines on the PR.

Do not touch `testgen`. `terrain_output_is_pinned` guards the fixture the
mesher benches compare against, and changing it retires every earlier number.

## Tests

- A Chunk with uniform light produces exactly the quad count it did before, so
  the merge key costs nothing where light is flat.
- Two faces differing only in light do not merge; two agreeing in both do.
- Every vertex of a quad carries the same light, and it is the light of the
  Voxel in front of the face, not of the Voxel the face belongs to.
- `light.len() == positions.len()`.
- A face on a Chunk boundary is lit from the neighbour's light, not from 0.
