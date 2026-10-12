# The blended pass in the app

Status: ready-for-agent
Blocked by: 03

Spec decision 6. The first time a player sees any of this.

## What to build

- A second entity per Chunk for the `blended` layer, with a `ChunkMaterial` whose
  base is `AlphaMode::Blend`, despawned and respawned with the opaque one.
- Palette `w` is alpha; glass and water get an alpha, leaves stay 1.
- The hotbar gains glass, leaves and water Voxels.
- Read the Bevy 0.20 source for whether a blended mesh casts shadows and what
  it contributes to SSAO and TAA, and record the answer in the ticket.
- Screenshot from the `dev` capture (`TESSERAE_SCREENSHOT_OFFSCREEN=1`) of a
  glass house, a leaf canopy and a pool, with the order artefact looked for and
  the choice among the spec's three options recorded.

## Acceptance criteria

- A Chunk with no blended Voxel spawns exactly one entity, as now.
- Voxel size scales the blended mesh like the opaque one.
- The screenshot is shown, and `./scripts/cargo-ci` passes.

## Comments
