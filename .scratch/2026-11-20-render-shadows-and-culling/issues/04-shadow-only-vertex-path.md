# 04: A shadow-only vertex path (1d)

Status: wontfix
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
- [ ] Captures against the commit before it differ no more than two runs of
      that commit do (the spec's image rule).

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

Gate (2026-11-21), against 03, release `profiling` builds, load 5 to 6: fails,
reverted. Its target did not move beyond run spread in any fixture.

| GPU series, p50 ms (03 → 04) | shadow span | GPU frame |
| --- | --- | --- |
| flight | 0.155 → 0.152 | 3.37 → 3.41 |
| active | 0.151 → 0.150 | 3.51 → 3.43 |
| transparency | 0.155 → 0.149 | 3.51 → 3.51 |
| stationary | 0.155 → 0.155 | 2.99 → 2.97 |
| sprint-flight | 0.166 → 0.167 | 3.45 → 3.43 |

Frame time is no worse: a tie-break of 03, 04 and 05 run back to back
(03-04-05-05-04-03, timing only, pooled p50) gave 2.98 → 3.02 ms in `flight`,
3.14 → 3.27 in `active`, 3.18 → 3.17 in `transparency` and 2.77 → 2.72 in
`stationary`, mixed in sign and inside the spread. After 03 the cascades hold
no cutout past 25 m, and the opaque shadow pass's vertex decode is too small a
share of the frame for a lighter decode to show. Reopen if 06 or 07 leaves the
shadow pass's vertex work a measured cost.
