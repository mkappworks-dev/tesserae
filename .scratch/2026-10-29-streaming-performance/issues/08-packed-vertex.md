# 08: Pack the Chunk vertex

Status: ready-for-agent
Blocked by: 01

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-packed-vertex`.

## What to build

Today a vertex is 40 B: position and normal `f32x3`, UV `f32x2`, material and
light `u32`.

- Pack it to 16 B, decoded in `chunk_vertex.wesl` and the prepass. Normals are
  an index into a table built from the `ShapeTable`'s face normals.
- Positions must represent every Shape vertex and fluid level exactly. If 16 B
  cannot, use 20 B; do not trade exactness. Leave room for positions local to
  a region of Chunks, which 09 needs.
- Each mesh carries an explicit `Aabb`, since Bevy cannot compute one from a
  packed position.
- Not per-quad packing (Voxy's 8 B a quad, pulled by the vertex shader): it
  leaves Bevy's standard mesh path for the prepass, shadows and batching. It
  becomes a follow-up ticket only if, after this one, upload or vertex cost
  still dominates in 01's attribution.

## Acceptance criteria

- [ ] A test decodes every Shape and fluid level to today's positions, normals
      and UVs exactly.
- [ ] Main-thread mesh upload time or GPU vertex time passes the spec's effect
      gate.
- [ ] Frustum culling still culls: 01's drawn-Chunk counts match the baseline.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.
