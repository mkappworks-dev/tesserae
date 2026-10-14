# Tune the fog against captures

Status: ready-for-agent
Blocked by: 02

Spec decision 2's curve and defaults, decision 4's boundary check, and
invariant 5 (the cut and the tint).

## What to build

Nothing new: choose the curve and the defaults from what is seen, and prove the
two things fog could break.

- **The curve.** Render `ExponentialSquared` (the spec's choice) and
  `Exponential` on the same views, with the same `reach_opacity`, and keep the
  one that looks right: the ground at the Player's feet unhazed, the coarse
  rings softened, a horizon that reads as chosen. A throwaway local edit for the
  comparison, not committed. If the choice changes, the ticket 01 formula and
  its tests change with it, in this ticket's commit, with the reason.
- **`reach_opacity` and the colour.** Compare 0.75, 0.85 and 0.92 at least. The
  far plane clips terrain at `lod_distance_m` plus a Cell, so a low opacity
  shows a clip edge and a high one hides the 4 km the LOD step built: weigh them
  in words and keep one. If the dark default sky makes distance read as gloom
  rather than haze, say so and, if it needs it, change the clear colour and the
  fog colour together to something that reads as air, and say that it is a
  change of the sky's colour only.
- **No step at the camera boundary.** At full resolution, across the join of
  level 0 and level 1 (192 m out) look for a change in the fog gradient or a
  line where the near camera ends. Read a row of pixels across it if the eye is
  not enough.
- **The cut and the tint.** The near-ground view (Eye about 64 m, 7 m short of a
  lattice corner, looking across it) and a straight-down view at 150 m, with
  `TESSERAE_LOD_TINT=1`: no coarse block stands over the level 0 horizon, no
  sky shows where terrain should be (`TESSERAE_SCREENSHOT_MAGENTA_SKY` over the
  views the LOD outcome used), and the tint still shows the rings through fog.
- **Terrain at the reach.** A view that has level 5 terrain around 4 km in it:
  state whether it is visible and how faint, as a number if it can be read from
  pixels (terrain against sky, 8-bit sRGB).

## Acceptance criteria

- Every capture is in `/tmp/tesserae-fog-review/` only, looked at, and described
  in a comment on this ticket in words: what the view is, what is seen, what was
  decided. No image is staged or committed, and the folder is deleted at the end
  of the whole job, not here.
- The defaults in `FogSettings::default()` are the chosen ones, and the tests of
  ticket 01 still pass or were changed with the reason in the commit message.
- Mutation-check the tint and cut result the way the LOD step did: make the fog
  wrong in a way that should be seen (for example force the fog off in the far
  cameras only) and confirm the boundary check catches it. Say how.
- The Outcome section of the spec has this ticket's findings, including the
  numbers and what did not work.
- `./scripts/cargo-ci` passes.
- `Status: done` is set in the commit that finishes this ticket.

## Comments
