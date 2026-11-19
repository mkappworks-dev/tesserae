# 02: Quad records

Status: done

Spec: [Packed quads](../spec.md).

## What to build

Item 1. The mesher emits one record per quad; the Client stores 8 bytes a
quad in one engine-owned GPU buffer and expands corners in the vertex shader.

- Core: `Quad`, `MeshLayer { quads }`, `Quad::corners()`, `MeshLayer::soup()`;
  every pass writes records. ADR-0030.
- Server: `level_the_water` and `split_quads` work on records; `MeshSize`
  counts record bytes.
- Client: `ChunkQuads` (table, records, range), the quad buffer and its free
  list, render-world sub-range writes, placeholder meshes with an explicit
  `Aabb`, `MeshTag` as the range, the blended sort writing records, collision
  boxes from records, `mesh_bytes` from records.
- Shader: `chunk_vertex.wesl` reads the record from the quad buffer.

## Acceptance criteria

- [x] Every meshing test passes on `soup()`.
- [x] The exact-expansion test covers every Shape, fluid level, flame, Cross
      and Plane form, through the uploaded words.
- [x] Chunk view tests read each entity's range back from the replayed buffer.
- [x] Gated by ticket 06: captures identical, no worse frame p50/p95, mesh
      memory or mesh worker time.

## Comments

2026-11-17: written without measuring, while step 17 builds and benchmarks.
`scripts/cargo-ci` passes. A temporary test, not committed, ran the old mesher
beside the new one on terrain, every Shape in every Orientation soaked, flames,
seam walls and seeded random Chunks: the new records expand to exactly what the
old 16-byte packing drew. Not yet checked: the WGSL compiles only when the game
starts, so the shader, the captures and the gate wait for tickets 01 and 06.

2026-11-19: gated in ticket 06 against the base: held mesh memory -84%, but
frame p50 +8-13% and p95 +13-26%, from the GPU's depth prepass and shadow
passes. Fails the gate; the code stays while the regression is looked at.
`625f7a83` fixes the game-start panic the uncompiled pipeline had.

2026-11-19: with ticket 07's two fixes (the layer's `Aabb` from its records,
the placeholder drawn indexed) the branch passes against the base: frame p50
and p95 no worse in any fixture, held mesh memory -70%, mesh worker time
lower, captures at the base's own run-to-run level. Numbers in ticket 07.
