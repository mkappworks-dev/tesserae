# A Chunk mesh is one record per quad

Status: accepted.

The Server's mesher used to send the Client triangle soup: four vertices a quad
with `f32` positions, normals and UVs, a Material and a light word each, and six
indices, about 184 bytes a quad. The Client packed it to 16 bytes a vertex plus
the indices, about 88 bytes a quad on the GPU and as much again in the main
world. Every quad is an axis-aligned rectangle on the sixteenth grid or one of
two fixed flat forms, with one Material and one light level, so almost all of
that is redundant.

## The mesher emits records

A `ChunkMesh` layer is a list of `Quad`s: two 32-bit words of geometry, a
Material (or `FLAME`) and a `Light`. The geometry is one of three kinds:

- **a face**, kind 0 to 5 in `FACE_STEPS` order: its plane, biased by 16
  sixteenths so that a flame and coarse water may lie just outside the Chunk,
  its low corner and its size, all in sixteenths of a Voxel. Pass 1, pass 2,
  fluid, waterlogged fluid, seam walls and flames are faces;
- **a Cross**, kind 6: one diagonal of its cell, front or back;
- **a Plane form**, kind 7: one quad across the thin axis of its box, front or
  back, its plane rounded down to the sixteenth grid.

`Quad::corners()` is the one expansion every reader shares, with the corner
order, winding and UVs the old soup had. `MeshLayer::soup()` rebuilds the
triangles for readers that want them.

The layout of the two words is part of the contract: the Client uploads them as
they are, and `chunk_vertex.wesl` decodes them. The high word's top 14 bits are
left free for the Client's per-layer table of (Material, light) pairs, so that a
quad is 8 bytes on the GPU.

## Why records and not packed vertices

- One record a quad replaces four vertices and six indices on the Server, in the
  message, in the main world and on the GPU, where only a one-byte placeholder
  vertex is left for each of the six the draw runs.
- The renderer expands the corners in the vertex shader from the vertex index,
  so the quad's four corners are never stored anywhere.
- The Server's mesh jobs stop building an `f32` soup that the Client only
  re-packed.

Instancing was rejected: Bevy's `instance_index` indexes the mesh uniform, so an
entity cannot be drawn as one instance per quad. Packed vertices with an index
buffer, today's form, were rejected for their size.

## Consequences

- A Plane form box of odd thickness lies on the sixteenth below its centre,
  which is where the old 16-byte packing already drew it.
- Coarse water that `level_the_water` splits per cell restarts its UVs at the
  cell. The texture repeats per Voxel, so the image is the same up to rounding.
- Any new kind of geometry needs a kind, or must be a face. Three bits of kind
  leave no room for a ninth.
- A layer with more than 16,384 distinct (Material, light) pairs is stored by
  the Client with the pair inline, 12 bytes a quad, so the contract holds for
  any Chunk.
