# Seams and the 4 km capture

Status: done
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

The hole arithmetic, the camera per level and the two suns are in the spec's
decisions 4 and 8; what the captures showed and the measurements are in its
Outcome. In short:

- Bound test: 60 columns at each of levels 1 to 5; three mutants killed (no
  height tie-break, solid threshold 3/4, solid threshold 1/4).
- Captures (offscreen, 1080p): horizon and 150 m, plain and tinted, straight
  down, and five magenta-sky views counted for clear-colour pixels under
  terrain. No crack, no flicker between two frames, no coarse bump; skirt not
  built.
- Reach: level 5's furthest drawn Chunk spans 4061 to 5174 m from the Eye.
- `BIAS_FRACTION` is gone (it could not work with the overlap under a finer
  camera); the far cameras' `near` stays at a quarter of the level-0 radius.
- Frame cost: 5.6 ms median against 4.0 ms for ticket 06's two cameras.
Added after the final review: a near-ground capture (Eye at y = 64 m, looking
across a lattice corner) showed coarse blocks over the finer horizon, so the
shader cuts a coarse fragment inside the finer level's guaranteed cylinder (spec
decision 9, Outcome). The first version showed the sky from 150 m straight down;
the magenta probe caught it and the cut is now bounded in height too.
