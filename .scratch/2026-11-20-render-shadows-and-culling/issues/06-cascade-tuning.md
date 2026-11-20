# 06: Cascade tuning (1b)

Status: ready-for-agent
Blocked by: 03, 04, 05

Spec: [Shadows and culling](../spec.md).

## What to build

Bevy's defaults: four cascades with far bounds 10, 25, 61 and 150 m, overlap
0.2, a 2048 map, Gaussian filtering. Each cascade is a render pass, a cull and
two parameter builds; the overlap band samples two cascades in the main pass.

- Measure a few candidates by the GPU series alone: three cascades, a 1536 or
  1024 map, a shorter far bound, overlap 0. Choose against today's 192 m view
  distance; step 22 re-checks it when it raises the default.
- Captures of each candidate shown to you; you pick, or keep the defaults.
- The chosen config is set on the sun with one line of why; ticket 03's far
  cascades are revisited if the count or bounds change.

## Acceptance criteria

- [ ] Candidates' GPU numbers and captures in Comments.
- [ ] Your choice gated by 01's method; numbers in Comments.
