# Collision taller than the cell

Status: done
Blocked by: 01

Spec decisions 7 and 8: gap (b), a fence you cannot jump. Independent of
connected shapes, and useful on its own for any Shape (a tall post, a sign).

## What to build

- `CollisionBox { min: [i8; 3], max: [i8; 3] }`, validated to `-16..=32` per
  axis with `min < max`; `From<NodeBox>`. `ShapeDef::collision` becomes
  `Option<Vec<CollisionBox>>`; `ShapeTable::collision` returns
  `&[CollisionBox]`. `ShapeError::BadBox` covers the new range.
- `rotate_box` rotates the signed range correctly (a box overhanging +Y,
  rotated onto its side, overhangs sideways).
- `ShapeTable::overhang() -> [u8; 6]`: per face, in Voxels (0 or 1), the
  furthest any registered collision box reaches past that face. All zero for the
  builtin table.
- `collide::clip` widens its cell range by `overhang`: on each axis the low end
  by the positive face's, the high end by the negative face's.
- `obstacles` returns a borrowed `&[CollisionBox]` (the table's slice, or a
  static one-box slice for an unloaded cell) instead of a `Vec`. The allocation
  per visited cell goes away; the sweep's results do not change.
- `in_metres` and the sweep's box tests take `CollisionBox`.

## Tests

- A fence-like test Shape (`[6,0,6]..[10,24,10]`): a body falling onto it at
  Voxel size 1.0 comes to rest at 1.5 m, and one walking at y = 1.0 is blocked
  horizontally by the part above the cell. A body whose feet are at 1.5 m walks
  over.
- The same scene at Voxel size 0.25 gives the same result scaled in metres
  (ADR-0003), i.e. rests at 0.375 m.
- A box overhanging **down** is found from the cell below; a rotated overhang
  is found sideways; a box overhanging nothing visits exactly the cells it did
  before (assert the range, so zero overhang stays zero cost).
- Registration rejects a box outside `-16..=32`.
- All existing `collide` tests pass unchanged.

## Done when

`./scripts/cargo-ci` passes; the sweep benches from 01 are no slower, and faster
on the paths that stop allocating - the PR says by how much.

## Comments

Done. `overhang()` folds over all 24 Orientations, so the first oriented Shape
that overhangs widens all six faces (spec decision 7 now says so); a test pins
it. Disabling the widening fails the three tests that need it. The sweep
benches could not be compared, since the machine was too loaded to take a
baseline (ticket 01); the allocation removal in `obstacles` is therefore
unmeasured.
