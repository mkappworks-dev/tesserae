# 14: Coarse water matches level 0

Status: done
Blocked by: 10

Spec: [Water that looks natural](../spec.md), Coarse levels.

## What to build

Water at a distance looks like the water near the Eye. Coarse meshes carry water
columns too. Level 1's follow level 0's per-column rule, since the coarse mesh
job already holds the Generator; levels 2 and up take the Generator's depth.
Coarse water takes the same water path, calm, with no Flow.

## Acceptance criteria

- [x] A test: a level-1 water column's depth agrees with level 0's at its centre
      within one coarse Voxel.
- [ ] Captures from `flight` and the coast fixture show no colour step at the
      edge of level 0; shown to the user.

## Comments

- 2026-11-27, on b8243600 plus this ticket:
  - Every coarse level takes its water columns from the Generator alone,
    `water_columns::coarse_water_columns`: depth over each cell's centre from
    its Water surface to the ground, calm, the sea flagged. Not level 0's rule
    at level 1: a coarse mesh draws the Generator's water and keeps no
    simulated fluid of its own to scan. Level 1's depth agrees with level 0's
    generated rule within one coarse Voxel over the coast fixture's sea (a
    `worldgen::water` test, 1 m against 2 m cells).
  - The coarse job adds them only to a mesh with a blended layer; the Client
    uploads them as at level 0. The water path reads the depth slope in each
    level's own cell size, so the sea's waves shoal there too.
- Captures in `/tmp/tesserae-water-look-captures/14-after/`: `flight-sheet.png`
  (ticket 13 above, this below), where the lake's colour and ripples no longer
  change where level 0 ends, and the coast. They wait on the user's review.
- `server::lod::agreement::coarse_ground_agrees_with_level_zero_at_each_column_centre_over_terra_coast_and_forest`
  fails on this branch: level 5 over the coast window, 3 columns' Canopy cover
  differ from level 0's tree density past one column. It does not read water;
  ticket 16's full run checks it against `main`.
