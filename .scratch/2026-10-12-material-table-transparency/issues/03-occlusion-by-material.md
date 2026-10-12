# Occlusion by Material and the two-layer ChunkMesh

Status: done
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

**What differs from the ticket as written.**

- `Neighborhood` records the Voxels of its seven palettes (`Chunk::kinds`) and
  `greedy_mesh` takes `&Tables`, as does `mesh_chunk`.
- `emits_no_faces` also requires the uniform Material to hide itself, which the
  ticket missed: a Chunk of leaves draws every face inside it. A neighbour that
  is the identical Voxel reuses that answer, so the declined case costs fewer
  instructions than before the table, not more.
- The client uploads `opaque` only. A Chunk with a blended layer and nothing
  opaque is despawned; both are 04's to change.
- The hole-free test states the rule itself, from the spec, and applies it at
  Voxel boundaries only; a first draft applied it between sub-cells of one Voxel
  and called a leaf's interior exposed. Forcing `hides` to `true` fails six tests.

**Measured** (retired instructions per call, harness against `f232c4e`):
`greedy_mesh` terrain -1.1%, solid -1.7%, checkerboard +0.0%; `mesh_chunk`
straddling -0.7%, declined_enclosed 142 to 120. New, no baseline: terrain under
water 12.9 M, with a canopy 13.3 M, a glass room 8.8 M, against 9.8 M for the
terrain alone.

