# Chunk streaming: three radii, a budget, and hysteresis

Status: done
Blocked by: 02, 03

The Server now owns World state and ticks. This ticket makes it load and evict
Chunks around the anchor the Client nominates, and hand finished meshes to the
Client.

## Task

Add `crates/tesserae/src/server/streaming.rs`.

### The intent channel

`StreamingAnchor`, a marker `Component` defined here. The Client puts it on its
camera; the Server reads that entity's `Transform`. The Server publishes the
intent it accepts, which is why the type lives on this side.

### The outbound channel

Two `Message` types, registered with `app.add_message`:

- `ChunkMeshReady { coord: ChunkCoord, mesh: ChunkMesh }` — a
  `tesserae-core` type with no Bevy in it. Sent even when the mesh is empty, so
  the Client has one rule: empty means despawn.
- `ChunkUnloaded { coord: ChunkCoord }`.

Messages, not observers: this is batch work read at a fixed point.
`RunFixedMainLoop` runs before `Update`, so a message the Server writes on a
tick reaches the Client in the same frame.

### The radii

`StreamingSettings`, a `Resource`, in metres: view distance, vertical extent,
and a hysteresis distance. Derive three radii in Cells from them:

- `mesh_radius` — Cells the Client gets a mesh for
- `load_radius = mesh_radius + 1` — face culling reads one Voxel across each
  face, so a Chunk meshed before its six neighbours exist emits a skin of faces
  buried in the neighbour's rock
- `evict_radius = load_radius + hysteresis`

The ordering is load-bearing, not defensive: because eviction trails loading by
a Cell, an evicted Chunk is never adjacent to a live mesh, so eviction cannot
leave a stale face behind and needs no re-mesh of its own. Comment it — it
reads like redundant slack.

### The tick

Precompute, once per radius change, the Cell offsets inside the load volume
sorted by squared distance. Walking that in order from the anchor Cell gives
nearest-first without sorting a few thousand coordinates every tick.

Then, per tick:

1. Evict loaded Chunks beyond `evict_radius`; send `ChunkUnloaded` for any the
   Client holds a mesh for.
2. Generate missing Chunks inside `load_radius`, nearest first, up to the
   budget. Mark the new coordinate and its six neighbours for re-meshing — an
   arriving neighbour changes the faces of a Chunk already meshed.
3. Mesh dirty Chunks inside `mesh_radius` whose six neighbours are all present,
   nearest first, up to the budget, via `mesh_chunk` from ticket 02. Clear the
   dirty flag and send `ChunkMeshReady`.

`StreamingBudget`, a `Resource`: Chunks generated and Chunks meshed per tick.
Ticket 06 measures the defaults; until then put a guess in with a `TODO`
naming ticket 06, and do not pretend it is measured.

## Tests

The ring arithmetic is the part worth testing headlessly, so keep it in free
functions that take an anchor and the settings rather than in system bodies.

- A coordinate inside `mesh_radius` is also inside `load_radius`, for a range
  of view distances including ones that do not divide the Cell size.
- `evict_radius` exceeds `load_radius` by at least one Cell, so no evicted
  coordinate is adjacent to any coordinate inside `mesh_radius`. Assert it over
  the whole volume rather than trusting the arithmetic.
- The offset table is sorted by distance and contains exactly the load volume.
- Radii change with `voxel_size`: halving it halves the Cell size, so the same
  view distance in metres doubles the radius in Cells. ADR-0003.
- A Chunk is not meshed while any of its six neighbours is absent.

## Done when

`cargo test --workspace` and `cargo lint` are clean, and `cargo dev` with the
Client from ticket 05 shows terrain appearing ahead of the camera and
disappearing behind it.

## Comments
