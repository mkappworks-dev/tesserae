# Leaves cut out, before textures

Status: done
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

Built, with the plan's routing changed.

- **A third mesh layer, not the opaque one.** For the holes to show in shadows
  the material has to discard in the depth and shadow passes too, which means
  an alpha-mask material; putting leaves in the opaque layer would have put
  every terrain fragment on a pass that may discard, which costs early depth
  and, on Apple GPUs, hidden-surface removal. So `ChunkMesh` has `cutout`, the
  merge key carries two bits of render class, and the Client spawns a third
  entity per Chunk on an `AlphaMode::Mask` material. The merge key's unpacking
  in `push_merged_quad` moved with it; a grep, not a test, caught that it would
  otherwise have read the Material one bit off.
- **The pattern.** Four by four holes a Voxel, about a third open, on the face's
  UV, which counts Voxels of its own level, so coarse leaves have coarse holes.
  In the main pass it sets alpha to zero and the mask's test discards; a new
  `chunk_prepass.wesl` cuts the same holes in the depth and shadow passes.
- **The prepass vertex layout.** `specialize` gave every pass the main pass's
  layout (normal at 1, UV at 2), and `bevy_pbr`'s prepass vertex shader reads
  the UV at 1 and the normal at 3. Harmless while nothing in the prepass read
  them; the cutout's holes do, so prepass pipelines now get `bevy_pbr`'s layout.
- Tests: leaves land in the cutout layer and nowhere else, and two leaves still
  draw the faces between them (core); a leaf mesh spawns one masked entity that
  casts a shadow, a replacement without leaves despawns it, and an unload
  removes it (Client).
- Capture: the gallery's canopy shows its inner faces through the holes, and
  its shadow on the plinth has gaps. A scene without leaves is unchanged
  against `origin/main` but for the overlay text, so opaque terrain and its
  shadows did not move with the prepass layout.

