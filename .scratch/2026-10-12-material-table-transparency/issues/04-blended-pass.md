# The blended pass in the app

Status: ready-for-agent
Blocked by: 03

Spec decision 6. The first time a player sees any of this.

Written for level 0 and widened after LOD (step 6) merged: the Client's entity
map is keyed by `(level, coord)` and `upload_meshes` serves every level, so the
blended entity is built for every level by the same code. What LOD adds beyond
that, compositing a blended surface across the per-level cameras, is ticket 06.

## What to build

- `upload_meshes` (`crates/tesserae/src/client/chunk_view.rs`) stops skipping a
  Chunk whose `mesh.opaque` is empty and uploads `mesh.blended`. A second entity
  per `(level, coord)` for the `blended` layer, with a `ChunkMaterial` whose base
  is `AlphaMode::Blend`, despawned and respawned with the opaque one. It has the
  opaque entity's placement, scale (`voxel_size_at(level)`), `MeshTag(level)` and
  `RenderLayers`, so the level's camera draws it, the shader's `cut` applies to
  it and the right sun lights it.
- Palette `w` is alpha; glass and water get a colour and an alpha, leaves stay 1.
  Materials 5 to 7 must stop being the magenta default.
- The hotbar gains glass, leaves and water Voxels.
- Read the Bevy 0.20 source and record the answer in this ticket:
  - whether a blended mesh casts shadows (`bevy_pbr` `render/light.rs` appears to
    queue `Blend` meshes into the shadow pass and `prepass/mod.rs` to skip them
    in the depth prepass; confirm) and decide `NotShadowCaster` or not. Only the
    sun on layer 0 casts, so the decision is level 0's. The flood already dims
    sky light under water by 2 levels, so a full shadow on top double-counts.
  - what it contributes to SSAO and TAA. Neither is on a camera today, so this
    is a note for step 9.
- Screenshot from the `dev` capture (`TESSERAE_SCREENSHOT_OFFSCREEN=1`) of a
  glass house, a leaf canopy and a pool, with the order artefact looked for.
- The choice of how to sort, recorded in decision 6 of the spec. Bevy sorts
  `Blend` meshes per entity by mesh centre, per camera; that is wrong inside one
  Chunk. Options, cheapest first: accept it; re-sort a Chunk's blended quads on
  camera movement (a main-thread index rewrite, to be measured against the worst
  tick, level 0 only); or `OrderIndependentTransparencyPlugin`. Read the OIT
  source before choosing: the spec calls it weighted-blended, and it is a
  per-pixel fragment linked list with a resolve pass, which **panics on a camera
  with MSAA on** (the Client sets none, so it has Bevy's default), needs the
  chunk fragment shader to call the OIT draw function instead of returning a
  colour, and adds a resolve pass per OIT camera. Choose from the screenshot, and
  correct the spec's wording in the same commit.
- The stale "ticket 04" comment in `upload_meshes` is removed, and the LOD
  spec's "No blended draw call" loose end points here.

## Acceptance criteria

- A Chunk with no blended Voxel spawns exactly one entity, as now. A Chunk with
  only blended geometry spawns one (the opaque-empty skip is gone), with both
  spawns two, with neither none. At level 0 and at a coarse level.
- A mesh replaced by one that drops a layer despawns that layer's entity and
  frees its mesh handle; unloading removes both; an upload and its unload in one
  frame leave nothing behind.
- Voxel size scales the blended mesh like the opaque one, at every level.
- The blended material's alpha mode is `Blend`, the opaque one's is unchanged, and
  the palette alpha of glass and water is below 1 and of every other Material 1.
- Mutation check: putting the skip back fails the blended-only test.
- A scene with no blended Voxel is unchanged: same capture as `main`, and the
  worst main-thread tick within noise (`MainThreadCost`).
- The screenshot is shown, and `./scripts/cargo-ci` passes.

## Invariants

The Client reads the `blended` layer of the `ChunkMesh` it already receives and
spawns render entities; nothing new crosses from the Server and no World state
is touched (ADR-0002). Alpha and colour are app-side appearance and never a
`MaterialDef` field, so `tesserae-core` is unchanged.

## Comments

2026-10-14: widened to every level, and the sorting and shadow decisions made
explicit, when ticket 06 was written. 06 holds what only LOD forces.
