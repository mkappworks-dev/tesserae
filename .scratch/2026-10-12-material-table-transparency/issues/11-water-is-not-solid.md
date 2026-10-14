# Water is not solid

Status: done
Blocked by: 01

`collide` reads Shapes only, so a sweep stops on water and the edit ray hits
its surface. Decided: water is not solid.

## What to build

- `MaterialDef::solid`. Water is not; every other built-in is. An id the table
  does not know stays solid, like its other defaults.
- The AABB sweep passes through a non-solid Voxel.
- The edit ray passes through it to what is behind. Break never removes water
  (the ray does not stop on it); Place may fill a cell that holds water,
  replacing it.
- `tesserae-core` stays free of Bevy; the Server's edit path is the only writer.

## Acceptance criteria

- Tests: a sweep through a pool reaches the bed; the edit ray from above a pool
  breaks the bed, not the water; Place into a water cell replaces it; Place into
  a stone cell is still `Occupied`.
- `./scripts/cargo-ci` passes.

## Comments

Built as specified. `sweep` and `raycast` take `&Tables` instead of
`&ShapeTable`, the single value spec decision 7 says a native consumer takes; a
non-solid Voxel contributes no collision box and no selection box. The edit
path's Occupied test asks `solid` instead of "not air". A row is five bytes
now, not four; the test that pinned four is updated and the spec says why. Four
new tests (three in `collide.rs`, one in `edit.rs`); putting the old Occupied
rule back fails the edit one. The collide bench builds against `Tables`.

