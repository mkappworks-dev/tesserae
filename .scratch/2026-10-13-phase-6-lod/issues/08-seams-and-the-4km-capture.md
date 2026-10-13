# Seams and the 4 km capture

Status: ready-for-agent
Blocked by: 06

Spec decision 9, and the step's "Done when".

## What to build

- The surface bound test: over a sweep of columns and levels 1 to 5, the coarse
  level's top solid Voxel, in metres, is within one coarse Voxel of
  `Worldgen::surface_height_m`. Mutation-check it (wrong tie-break, lost surface
  Voxel).
- Capture at least: the default vantage looking at the horizon; a vantage
  climbed to about 150 m; and each with `TESSERAE_LOD_TINT=1` so the rings are
  visible. Look at every image. Inspect the ring joins at full resolution for
  cracks, flicker (take two frames), and coarse bumps through the fine layer.
- Tune `BIAS_FRACTION` and the far camera's `near` only against what is seen.
- If a crack is visible, build the fallback skirt (spec decision 9) rather than
  widening the overlap. If none is visible, say so and do not build it.
- Confirm the 4 km reach: terrain is drawn out to about 4096 m (the capture's
  report lists the furthest delivered level-5 Chunk).

## Acceptance criteria

- Screenshots are in the PR description's evidence and shown to the user; the
  commit message or Comments says what each shows.
- `./scripts/cargo-ci` passes.

## Comments
