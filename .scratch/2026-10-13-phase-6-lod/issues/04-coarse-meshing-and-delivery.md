# Coarse meshing and delivery

Status: ready-for-agent
Blocked by: 03

Spec decisions 3, 5 (mesh half) and 7.

## What to build

- Mesh job for a coarse Chunk: `Arc<Chunk>` for it and its six same-level
  neighbours, a `ChunkLight::filled(Light::new(MAX_LIGHT, 0))` for all seven,
  the `Tables`; returns a `ChunkMesh` through the existing `mesh_chunk`. No
  change to `mesh.rs`, `light.rs` or `chunk.rs`.
- Dispatch only for Chunks in the level's mesh shell (outer minus hole), nearest
  first, requiring all six neighbours held. No settled-light gate (there is no
  light).
- Landing: delivered if inside `outer.grown(hysteresis)`; a result whose Chunk
  was dirtied during the flight is delivered anyway and stays dirty. Retire
  meshes that leave the keep shell (outside `outer.grown(h)` or inside the hole
  shrunk horizontally by `h`).
- New messages `LodMeshReady { level, coord, mesh }` and
  `LodUnloaded { level, coord }` in `server/streaming.rs`, written only by the
  tick. Level 0's messages are unchanged.
- `StreamingStats` (dev) gains per-level held / meshed counts for the capture
  report.

## Tests

- At quiescence the delivered set per level is exactly the mesh shell's Chunks
  that have six neighbours, and each delivered mesh equals a fresh `mesh_chunk`
  of the store with the synthetic light.
- No mesh is delivered before its six neighbours are held.
- Every vertex light on a coarse mesh is `Light::new(15, 0)`.
- Eviction safety, the module-doc argument per level: teleport the anchor with
  Jobs outstanding; no mesh lands whose neighbour was evicted.
- The hole: Chunks inside it are not meshed, Chunks inside the previous level's
  extent but outside the hole are (the overlap), and walking onto a coarse Chunk
  releases it only past the hysteresis.
- A level-1 mesh contains no shaped-Voxel geometry (`shaped_voxels == 0`).

## Acceptance criteria

- `./scripts/cargo-ci` passes.

## Comments
