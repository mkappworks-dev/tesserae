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
