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
they are, and `chunk_vertex.wesl` decodes them. `tesserae_core::mesh::layout`
holds it, and reaches the shader as shader defs. The high word's top 14 bits are
left free for the Client's per-layer table of (Material, light) pairs, so that a
record is 8 bytes on the GPU.

## The draw

Every Chunk layer's header, table and records live in one engine-owned storage
buffer, a range each. The layer's entity keeps a `Mesh3d` whose mesh is a
placeholder that no pipeline reads: four one-byte vertices a quad and the
indices `0 1 2 0 2 3` per quad, two bytes each up to 65,536 vertices and four
past them. The vertex shader takes the quad from the index over four and the
corner from the index modulo four, so the GPU can reuse a corner's vertex
shading across the quad's two triangles, as it did with the old index buffer.

The pipelines keep one vertex buffer slot with no attributes and a stride of 0:
Bevy's specializer keys a pipeline by `buffers[0]`, so the slot must exist, and
wgpu wants a stride that is a multiple of four, which the one-byte placeholder
is not.

A quad is then 24 bytes on the GPU: 8 of record, 4 of placeholder vertices and
12 of indices. A layer past 65,536 vertices takes four-byte indices, 36 bytes a
quad.

## Why records and not packed vertices

- One record a quad replaces four vertices and six indices on the Server, in the
  message and in the main world; on the GPU the record and its placeholder are
  24 bytes where the packed vertices and indices were about 88.
- The renderer expands the corners in the vertex shader from the vertex index,
  so the quad's four corners are never stored anywhere.
- The Server's mesh jobs stop building an `f32` soup that the Client only
  re-packed.

Instancing was rejected: Bevy's `instance_index` indexes the mesh uniform, so an
entity cannot be drawn as one instance per quad. Packed vertices with an index
buffer, the old form, were rejected for their size. A non-indexed placeholder,
six one-byte vertices a quad and 14 bytes on the GPU, ran the vertex shader six
times a quad and made the depth prepass and shadow passes slower than the old
form; the index buffer's 10 more bytes a quad buy that back.

## Consequences

- A Plane form box of odd thickness lies on the sixteenth below its centre,
  which is where the old 16-byte packing already drew it.
- Coarse water that `level_the_water` splits per cell restarts its UVs at the
  cell. The texture repeats per Voxel, so the image is the same up to rounding.
- Any new kind of geometry needs a kind, or must be a face. Three bits of kind
  leave no room for a ninth.
- A layer with more than 16,384 distinct (Material, light) pairs is stored by
  the Client with the pair inline, 12-byte records, so the contract holds for
  any Chunk.
- Each layer has its own placeholder. Shared placeholders in size buckets would
  save its 16 bytes a quad for more vertex work, the cost the indexed draw won
  back.
