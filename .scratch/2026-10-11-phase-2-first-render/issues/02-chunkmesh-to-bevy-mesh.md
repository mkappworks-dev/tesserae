# Convert `ChunkMesh` into a Bevy `Mesh`

Status: done
Blocked by: 01

## Task

In `crates/tesserae` — never in `tesserae-core`, which must acquire no `bevy`
dependency — convert a `ChunkMesh` into
`Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())`
with:

- `Mesh::ATTRIBUTE_POSITION`, `ATTRIBUTE_NORMAL`, `ATTRIBUTE_UV_0`
- a custom `MeshVertexAttribute` holding the per-vertex Material index
- `Indices::U32`

`ChunkMesh` positions are in Voxel-index space. Scale them by the World's
`voxel_size` here, so the `Aabb` Bevy derives, frustum culling and shadow
bounds are all already in metres. Do not hardcode a one-Voxel-is-one-metre
assumption anywhere.

## Done when

Tests cover: every attribute the chunk material reads is present; positions
scale with `voxel_size` and a 32-Voxel Chunk at 0.25m fits an 8m Cell; normals
and UVs pass through unscaled; the Material index stays an integer rather than
becoming a float. These run headlessly — CI has no GPU.

## Notes

Bevy 0.20 added opt-in per-attribute vertex compression. `Mesh::new` leaves it
off, and the custom vertex shader in ticket 03 reads uncompressed attributes.
Quantising greedy-mesher positions is an obvious later win, and it will require
that shader to decompress.

## Comments
