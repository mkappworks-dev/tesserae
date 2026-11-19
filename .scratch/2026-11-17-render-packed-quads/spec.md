# Packed quads: Chunk mesh memory, upload and meshing cost

Status: ready-for-agent. Drafted 2026-11-17. Decisions Q1 to Q8 were settled
in a grilling round on 2026-11-17; the three questions left open wait on the
baseline (ticket 01).

Source: branch `tech/render-packed-quads`, cut from `main` at `d3bf6bc4`.
Builds on `.scratch/2026-11-01-gpu-pixel-cost/` (ticket 02's attribution) and
`.scratch/2026-10-12-material-table-transparency/` (blended sorting). Tickets:
[issues/](issues/).

## Problem Statement

A Chunk's mesh is triangle soup. The Server's mesher emits four vertices per
quad as `f32` positions, normals and UVs plus a Material and a light word per
vertex and six `u32` indices, about 184 bytes a quad, and sends it to the
Client. The Client packs it to 16 bytes a vertex (`Sint16x4` position and
face, `Uint16x2` UV, `Uint32` Material and light) plus the six indices, about
88 bytes a quad, uploads that, and keeps a copy in the main world. Every one
of those bytes is derivable from far less: a quad is an axis-aligned rectangle
on the sixteenth grid, or one of two fixed flat forms, with one Material and
one light level.

What it costs:

- Mesh memory on the GPU and on the CPU, which grows with the view distance
  and the LOD levels.
- Upload: every mesh that lands is copied to the GPU whole, and a blended mesh
  is re-uploaded whole each time its sort order changes.
- Message size between Server and Client, and the f32 soup the Server builds
  and drops for every mesh job.
- Pass 1's per-Voxel mask loop, the bulk of a mesh job on terrain. A flight
  profile from another session measured about 110 ms of mesh worker time per
  second of flight on one worker.

Nothing has been measured on this branch yet: other sessions' builds kept the
profiler's competitor check failing, and step 17 is building and benchmarking
now. The code is written first; ticket 01 takes the baseline, and every item
lands or is dropped on its A/B gate (ticket 06).

## Solution

1. **Quad records.** The mesher emits one record per quad: its geometry
   packed into two 32-bit words, its Material and its light. The Client
   builds a per-layer table of the distinct (Material, light) pairs, stores
   each record as 8 bytes with a table index in its spare bits, and puts the
   layer's table and records in one engine-owned GPU buffer. Each Chunk
   layer's entity keeps a `Mesh3d` whose mesh is a placeholder of six
   one-byte vertices per quad that no pipeline reads; the vertex shader finds
   the record from the vertex index and expands the corner. About 88 bytes a
   quad become 8 of record plus 6 of placeholder on the GPU, and 8 on the CPU.
2. **Binary greedy meshing** of pass 1: build each slice's face masks a row
   of 32 Voxels at a time with bit operations, then merge runs with
   trailing-zero scans. The records stay identical, which a test asserts
   against the old pass 1 kept as a test-only reference.
3. **Face-direction split**, a measured prototype: a Chunk layer split into
   six draws by face, the ones facing away from the Eye skipped. Only if
   ticket 01's numbers say draws are cheap enough and the sun's shadow pass
   can keep the faces it needs.
4. **Multi-draw indirect and GPU culling**, only after item 1 and only if
   ticket 01's numbers point there. On Metal, wgpu turns multi-draw into a
   loop of single draws, so it saves no draw calls.

The order is the expected payoff. Ticket 01's numbers decide whether it
holds.

## User Stories

1. As a player, I want terrain drawn exactly as it is today, so that the
   memory saving is invisible.
2. As a player, I want the view distance and the LOD levels to cost less
   memory, so that a larger World fits on a smaller Mac.
3. As a player, I want new terrain to land without upload stalls while
   flying, so that streaming stays smooth.
4. As a player, I want water and glass still drawn back to front from every
   angle, so that the sort's fix does not regress.
5. As a player, I want lantern light seams, smoothed Shapes, flames, vines
   and crosses unchanged, so that nothing the mesher draws is lost.
6. As a player, I want the camera to collide with terrain as it does today,
   so that third person still stops at walls.
7. As the developer, I want one record per quad from the mesher, so that the
   Server's mesh jobs and messages shrink too.
8. As the developer, I want a test that expands every record exactly as the
   shader does and compares it to the mesher's own vertices, so that the
   packing cannot drift.
9. As the developer, I want the binary mesher's output proven identical to
   the old one on the bench fixtures and on seeded random Chunks, so that
   speed never changes geometry.
10. As the developer, I want each item gated by an interleaved CPU-time A/B
    against the commit before it, so that a change that costs is dropped.
11. As the developer, I want pixel-identical before and after captures, so
    that a packing error is caught before a player sees it.
12. As a Mod author, I want nothing in the WIT contract or my Declarations to
    change, so that this work is invisible to Mods.

## Implementation Decisions

- **Core emits records (Q1).** `MeshLayer` holds `quads: Vec<Quad>`. A
  `Quad` is `geometry: [u32; 2]`, `material: u32` and `light: Light`, 16
  bytes in memory. `Quad::corners()` expands one exactly, and
  `MeshLayer::soup()` gives the old positions, normals, UVs, Materials, light
  and indices for the readers that want triangles: tests, the held-item
  preview, collision boxes. This changes what crosses from Server to Client,
  so it has an ADR (0034).
- **Three kinds of record.**
  - A face, kind 0 to 5 in `FACE_STEPS` order: an axis-aligned rectangle with
    its plane, low corner and size in sixteenths. Pass 1, pass 2, fluid,
    waterlogged fluid, seam walls and flames are all faces; a flame's plane
    lies a sixteenth outside its Voxel, and coarse water's plane where
    `level_the_water` puts it.
  - A Cross, kind 6: one of the two diagonals of a cell, front or back.
  - A Plane form, kind 7: one quad across the thin axis of its box, front or
    back. Its plane is rounded down to the sixteenth grid, which is what the
    16-byte vertex packing already drew.
  The corner order, winding and UVs are the ones each path wrote before, so
  `soup()` returns what the old mesher returned, except a Plane form box of
  odd thickness, whose plane the GPU never saw at the half sixteenth anyway.
- **The GPU record is 8 bytes (Q2).** The low word holds the kind and, for a
  face, a 10-bit plane biased by 16 sixteenths and a 9-bit corner on each of
  the two other axes; the high word holds the size, 9 bits an axis less one,
  and a 14-bit index into the layer's table of (Material, light) words, each
  as the old vertex attribute packed them. A layer with more than 16,384
  pairs stores 12-byte records with the pair inline instead; the stride is in
  the layer's header, so one shader reads both. This replaces Q2's "split the
  layer": it keeps one entity per layer.
- **One engine-owned quad buffer (Q3).** A `ShaderBuffer` with no CPU data,
  bound to all five terrain materials at binding 106. Each Chunk layer owns
  one range: a header (LOD level, table length, record stride), its table,
  its records. A first-fit free list hands out ranges; the Client writes a
  range with `RenderQueue::write_buffer` from a render-world system after the
  buffer is prepared, so a write lands in the frame whose extraction saw it.
  The buffer doubles when full: Bevy's `copy_on_resize` copies it on the GPU
  and invalidates the bind groups without touching the materials. A
  replaced or released range is freed at once, since the render world draws
  the frame it extracted.
- **The draw (Q3).** Each Chunk layer keeps its entity, `Mesh3d` and
  material. The mesh is non-indexed, one `Uint8` attribute, six vertices per
  quad, render-world only. `specialize` declares no vertex buffers, so the
  attribute is never read. `MeshTag` is the start of the layer's range; the
  shader reads the LOD level from its header, and the quad and corner from
  `vertex_index - mesh[instance].first_vertex_index`. The entity carries its
  own `Aabb` and `NoAutoAabb`, since the placeholder has no positions. The
  prepass shaders stay: defining them is what keeps the material bind group,
  and with it the quad buffer, bound in the shadow and depth passes.
- **The Client keeps the records (Q7).** A `ChunkQuads` component holds the
  layer's words as uploaded and its range. Collision boxes and the blended
  sort read it; the placeholder mesh is render-world only.
- **Blended sorting** writes the layer's records back in the new order into
  the same range. Nothing is re-uploaded but the records.
- **Acceptance (Q4).** Pixel-identical offscreen captures against the base,
  and an exact-expansion test: every record of every Shape, fluid level and
  flame, decoded as the shader decodes it, equals `Quad::corners()`, which
  equals the mesher's own vertices. Coarse lake cells that `level_the_water`
  splits restart their UVs at the cell; the texture repeats per Voxel through
  `fract`, so the image is the same up to rounding.
- **Gates (Q5).** Each item is gated by an interleaved CPU-time A/B against
  the commit before it: no worse frame p50 or p95, mesh memory or mesh worker
  time. A failing item is dropped, with the numbers in its ticket. The face
  split is a prototype first.
- **Binary greedy (Q6).** The old pass 1 stays as a `#[cfg(test)]`
  reference; a test asserts identical records over the bench fixtures plus
  seeded random Chunks.
- **No switch (Q8).** No old/new diagnostic switch; the old path is deleted
  in the commit that replaces it.
- **Open until ticket 01 (after measuring):** the order of items 2 to 4;
  per-layer placeholders or shared ones in size buckets; the quad buffer's
  initial size (1 MiB words, 4 MiB, for now) and growth policy (doubling for
  now).

## Testing Decisions

- Core: every existing meshing test runs on `soup()`, unchanged in what it
  asserts. A record round-trip test covers each kind at the extremes of its
  fields.
- Client: the exact-expansion test decodes every record as
  `chunk_vertex.wesl` does, from the words the Client uploads, and compares
  them with `Quad::corners()`; the Aabb test runs on the records. Chunk view
  tests replay the quad buffer's writes into a CPU copy and read each
  entity's range back, so landing, replacing, releasing and sorting are
  tested against what the GPU would hold.
- Binary greedy: identical records against the reference pass 1 on the bench
  fixtures and seeded random Chunks, translucent and wet included.
- Captures (ticket 06): `TESSERAE_SCREENSHOT_OFFSCREEN=1`, flight, active and
  transparency, before and after in one `/tmp` folder, never committed.

## Out of Scope

- Changing what the mesher merges, how it lights, or the render classes.
- Vertex pulling for the held-item preview, sprites and other non-terrain
  meshes.
- Moving the coverage discard, the drawn mask or the LOD rules of ADR-0019.
- Any Mod-visible change.

## Further Notes

- Bevy 0.20.1 facts this relies on, read from its source: `first_vertex_index`
  is set on every draw path and equals the draw's base vertex; on Metal draws
  are indirect (GPU preprocessing in culling mode); a `MaterialExtension`'s
  `specialize` runs for the forward, prepass and shadow pipelines;
  `ShaderBuffer` with `Uninitialized` data and `copy_on_resize` grows on the
  GPU and records the change in `RenderChangedShaderBuffers`; `NoAutoAabb`
  stops `calculate_bounds` from replacing an explicit `Aabb`.
- Shared size buckets for placeholders (one mesh per bucket, padding quads
  degenerate in the shader) would drop the 6 bytes a quad at about 12% more
  vertex work; ticket 01 decides.
