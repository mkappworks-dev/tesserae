# Occlusion by Material and the two-layer ChunkMesh

Status: ready-for-agent
Blocked by: 01

Spec decisions 3 and 4.

## What to build

- `hides(across, here)`: Opaque full cubes hide; a `SameMaterial` full cube
  hides the same Material; a shaped neighbour hides a rectangle only if its
  coverage contains it **and** its Material hides.
- `pass1::<TRANSLUCENT>`; `Neighborhood::new` records the Materials of its seven
  palettes so `greedy_mesh` decides the gate from them. `emits_no_faces` asks
  the table for the uniform-cube case.
- `ChunkMesh { opaque, blended, shaped_voxels }` over a `MeshLayer`; both passes
  route by the Material's `RenderClass`. The client builds its `Mesh` from
  `opaque` only in this ticket (nothing places a blended Material yet).
- Benches: `greedy_mesh/terrain_water`, `greedy_mesh/glass_room`.

## Acceptance criteria

- The brute-force hole-free scene test is extended to translucent Materials with
  a ground truth derived from the spec's rule; no exposed face is missing and
  every culled face is hidden by something that hides it.
- Glass beside glass leaves no face between them; glass beside different glass,
  and leaves beside leaves, keep it; stone beside glass keeps its face; glass
  beside stone loses its.
- A Chunk of terrain with no translucent Material meshes to exactly what it did
  before, quad for quad.
- No quad straddles layers, and a merged run never crosses Materials.
- `greedy_mesh/{terrain,solid,checkerboard}` and `mesh_chunk/*` unchanged
  against `before`, with numbers in the PR; `./scripts/cargo-ci` passes.

## Comments
