# Leaves cut out, before textures

Status: ready-for-agent
Blocked by: 04

Leaves draw as solid cubes because cutout needs an alpha test against a
texture, and textures are step 9. Decided: a procedural hole pattern now, which
step 9's textures replace. This reverses the spec's "`RenderClass` gets a
`Cutout` when that arrives and not before".

## What to build

- `RenderClass::Cutout`. Leaves are `Cutout`; the mesher sends `Cutout` faces to
  the opaque layer (they write depth and need no sort), so `ChunkMesh` keeps two
  layers.
- The chunk shader discards a fragment of a `Cutout` Material where a
  procedural pattern, in the face's own Voxel-local coordinates so it does not
  swim and is the same at any Voxel size, says there is a hole. Which Materials
  are `Cutout` reaches the shader as data, not as a hard-coded id.
- Shadows and the depth prepass see the holes (the alpha test runs there too).

## Acceptance criteria

- Leaves show gaps you can see through in a capture of the gallery's canopy,
  and the canopy's shadow has gaps too.
- Opaque Materials are pixel-identical to before (a scene without leaves
  against `main`).
- `./scripts/cargo-ci` passes.

## Comments
