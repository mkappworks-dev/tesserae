# A meshing entry point that can decline

Status: ready-for-agent
Blocked by: none

`greedy_mesh` takes a `Neighborhood`, so by the time it is called the padded
34-cubed copy has already been built — 39 thousand Voxel writes. Streaming a
3D ring asks for a mesh of mostly uniform-air Chunks, each of which pays that
build plus a full three-axis sweep to produce nothing.

## Task

Add to `crates/tesserae-core/src/mesh.rs`:

```rust
pub fn mesh_chunk(center: &Chunk, neighbors: [Option<&Chunk>; 6]) -> ChunkMesh
```

It answers with an empty `ChunkMesh` without building a `Neighborhood` in the
two cases where no face can be emitted, and otherwise builds one and calls
`greedy_mesh`. `greedy_mesh` and `Neighborhood` stay public and unchanged:
the benches and the mesher's own tests use them directly.

The two cases, both testable from `Chunk::uniform()` alone:

1. **The centre is uniform air.** Only Voxels inside the centre Chunk emit
   faces, so a uniform-air centre emits nothing whatever its neighbours hold.
2. **The centre and all six neighbours are uniform and all are full cubes.**
   Every face of the centre is occluded from the other side. A missing
   neighbour is not uniform-solid — it is unknown — so `None` disqualifies the
   case.

Case 1 does not depend on the neighbours at all, which is worth a comment: it
reads like an oversight next to case 2.

## Tests

- A uniform-air centre with solid neighbours meshes to empty.
- A uniform-solid centre with six uniform-solid neighbours meshes to empty.
- The same centre with one neighbour `None` does **not** mesh to empty, and
  emits exactly one Chunk face of quads.
- A uniform-solid centre with one *air* neighbour emits exactly one face.
- For a terrain Chunk, `mesh_chunk` and
  `greedy_mesh(&Neighborhood::new(..))` agree on positions, normals, uvs,
  materials and indices. The fast path must never change the output.

## Bench

Add to the `greedy_mesh` group: `mesh_chunk` on an air Chunk and on a
fully-enclosed solid Chunk, so the saving is a number in the PR rather than an
assertion.

## Done when

`cargo test -p tesserae-core` passes and the bench shows the two fast paths
costing orders of magnitude less than a meshed Chunk.

## Comments
