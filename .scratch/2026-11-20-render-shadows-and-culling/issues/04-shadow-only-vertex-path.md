# 04: A shadow-only vertex path (1d)

Status: ready-for-agent
Blocked by: 01

Spec: [Shadows and culling](../spec.md).

## What to build

The shadow pass runs the prepass vertex shader, which decodes everything the
colour pass needs. On macOS the opaque shadow pipeline has no fragment stage, so
only the position matters.

- `ChunkMaterialExtension::specialize` recognises the directional shadow key
  (`DEPTH_PREPASS`, orthographic, unclipped depth) and adds a shader def.
- Under it `chunk_vertex.wesl` computes the clip position only, through the same
  position decode as the full path: no (Material, light) table read, no normal;
  UVs and the Material slot only where the pipeline may discard (cutout's alpha
  holes).
- Launch the game after the change: no CI test compiles WGSL.

## Acceptance criteria

- [ ] One position decode serves both paths, so the expansion test covers the
      shadow path's positions.
- [ ] Gated by 01's method, the shadow span named by 01 as its target; numbers
      in Comments.
- [ ] Pixel-identical captures against the commit before it.

## Comments

Built (2026-11-20): `shadow_only` in `chunk_material.rs` adds `CHUNK_SHADOW`
for the directional shadow key. Under it the opaque shadow pipeline outputs the
clip position alone; cutout keeps normal, UV and Material for its alpha test
(the texture's face choice reads the normal, which the ticket left out). The
position comes from `expand_position`, which is `expand(..).position`: the
paint read, normal and UV are left for the compiler to drop.

Why not a decode of its own: splitting `expand` into a shared position helper
changed about 2,250 pixels at shadow edges of a 1280x720 capture, with or
without the new def. Metal compiles with fast math, and the local positions are
exact sixteenths, so the transform's rounding follows the code's shape.

Captures (dev, offscreen, three stationary views, six runs of the commit before
against six of this): not quite pixel-identical. 1, 8 and 31 pixels differ in
every run, by 1 (hill, down) to 7 (trees) levels, all from the opaque
shadow-only pipeline (bisected: cutout and colour are exact). Run-to-run noise
of the commit before itself is 100 to 1,600 pixels a view.
