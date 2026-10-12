# Reach is measured from the Eye

Status: done
Blocked by: none

Spec decision 9. Independent of 06 and 07.

## What to build

`server/edit.rs`:

- `Pending` gains `reach_m`. `receive` computes
  `d = origin.distance(eye)` (already needed for the origin check) and stores
  `EDIT_REACH_M - d`.
- `resolve` takes the reach instead of using `EDIT_REACH_M` directly.
- The doc on `EDIT_ORIGIN_TOLERANCE_M` and the module doc stop saying reach is
  10 m: the tolerance bounds staleness, not reach.

## Tests

- An origin 2 m behind the Eye strikes nothing 7 m along the ray, which an origin
  at the Eye does strike. Same ray, same ground.
- For a spread of origins within the tolerance, no struck Voxel is more than
  `EDIT_REACH_M` from the Eye.
- `d` = 0 behaves as before: every existing edit test passes unchanged.

## Then

`./scripts/cargo-ci`.

## Comments

Both new tests fail with the flat reach restored (checked by hand): an origin 2 m
below an Eye 9 m from the ground used to strike it from 7 m.
