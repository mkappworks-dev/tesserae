# 07: Columns

Status: done

Blocked by: 04

Spec: [Phase 18](../spec.md).

## What to build

The `column` kind: sugar cane, cactus, bamboo and kelp grow one Voxel at a time
up or down to `max-height-m`. A Sample that passes adds a Voxel at the end of
the run when the target is Air, or the fluid the Plant stands in, and the run is
under its height in metres. Removing a Voxel lets the run above regrow only
from the support rule, and a run whose base loses its support breaks from the
top, per ticket 06.

## Acceptance criteria

- [x] A column reaches its height in metres, the same at 1.0, 0.5 and 0.25 m.
- [x] Kelp stands in and grows through its fluid and not in Air; cane grows in
      Air and not into a solid.
- [x] A column stops at its cap and at an obstruction without dying.
- [x] Cutting the middle leaves the base to regrow and drops the cut Voxel's
      yield; the part above breaks by the support rule.

## Comments

- The row could not say kelp from cane, so `column-def` gains `stands-in:
  option<string>` (a fluid Material; load error otherwise) in both WIT copies,
  the TOML row, `Kind::Column` and resolution. Support is unchanged: a column
  stands on soil or its own Material, kelp included.
- Only the end Voxel of a run grows, so the rate is one Voxel per `mean-seconds`
  however tall the run. The cap is `ceil(max-height-m / voxel size)`.
- A new Voxel copies the Shape and Orientation of the Voxel that grew it; the
  Plant row names no Shape, the Player's placement chose it. A flat Shape with
  boxes has Voids, so `Occupants::place_solid` keeps the fluid and kelp is
  waterlogged where it grows; a Shape with no void displaces the water.
- Cutting the middle breaks the part above one Voxel a pass, up to
  `SUPPORT_PASSES` a look; a taller part finishes over the next steps.
- `grow` now returns a `Growth` (a list of `Change`s) so a tree can land many
  Voxels through the same landing loop in `fluids/plants.rs::sample_step`.
