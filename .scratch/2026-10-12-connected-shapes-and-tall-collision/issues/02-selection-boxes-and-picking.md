# Picking uses the drawn shape

Status: done
Blocked by: 01

Spec decision 9. Collision and the drawn shape are the same list for every
builtin, which hid that `raycast` asks the wrong one. Ticket 03 makes them able
to differ in height; this lands the distinction first so 03 cannot break
picking.

## What to build

- `ShapeTable::selection(voxel) -> &[NodeBox]`: the Shape's declared `boxes`,
  rotated by the Orientation the way collision already is. Stored per variant
  beside the collision list. (Ticket 04 gives it the `Connections` argument.)
- `collide::raycast` tests `selection` instead of `collision`.
- The hole-free oracle in `tests/meshing.rs` (`exposed_faces`) switches from
  `shapes().collision(v)` to `selection`, so it stays "the declared boxes"
  once collision can differ.

## Tests

- A Shape with `collision: Some(vec![])` (a pressure plate) is struck by a ray.
- A Shape whose collision is larger than its drawing (a thin drawn box with a
  whole-cell collision box, within `0..=16`; ticket 03 supplies overhang proper)
  is struck only where it is drawn.
- Every existing `collide` and `meshing` test passes unchanged: for the builtin
  Shapes `selection == collision`, and a test asserts that for all 24
  Orientations, so the switch is provably a no-op for them.

## Done when

`./scripts/cargo-ci` passes; `raycast` benches from 01 show no movement beyond
noise.

## Comments

Done. Reverting `raycast` to `collision` fails both new tests, so they catch the
regression. The `raycast` benches could not be compared: see ticket 01, the
baseline is still to be retaken on a quiet machine.

`server::edit`'s "occupied cell" test had used a no-collision plate as the thing
the ray fails to notice, which is exactly the bug. It now uses a post the ray
passes beside; the rejection it checks is unchanged.
