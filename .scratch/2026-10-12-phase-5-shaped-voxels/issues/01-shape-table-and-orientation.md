# The Shape table and Orientation

Status: done
Blocked by: none

Everything else in the step reads this, so it goes first and has no consumer
yet. Spec decisions 1 to 3.

## What to build

`tesserae-core/src/shape.rs`, exported from `lib.rs`, with no dependency:

- `NodeBox { min: [u8; 3], max: [u8; 3] }` in sixteenths, `0..=16`, and
  `ShapeDef { boxes, collision: Option<Vec<NodeBox>> }`.
- `Orientation`: one byte, `up * 4 + turn`, 0 to 23, with
  `Orientation::new(up_face, turn)`, `IDENTITY`, `up()`, `turn()` and a fallible
  `from_bits` that rejects 24 and above. `up` order is +Y, -Y, +X, -X, +Z, -Z so
  the identity is byte 0; the rotation is the up-rotation composed with a
  quarter-turn count about +Y, both right-handed. A Voxel read with an invalid
  orientation byte behaves as the identity.
- `ShapeTable::builtin()` with permanent ids `SHAPE_FULL_CUBE` 0 (already in
  `voxel.rs`), `SHAPE_SLAB` 1, `SHAPE_STAIR` 2, `SHAPE_POST` 3, and
  `register(def) -> Result<ShapeId, ShapeError>`. Reject: an empty box list, a
  degenerate or out-of-range box, more boxes than a documented maximum, and a
  256th Shape. Shape 0 can never be registered over.
- At registration, per Shape and per Orientation, derived from the rotated
  16x16x16 grid only: the six 16x16 face **coverage** masks, the 6-bit
  **seals**, the exposed-surface **quads** (greedy-merged rectangles on 1/16
  planes, each knowing its face, its plane and whether it lies on the cell
  boundary), and the **volume** in sub-cells. The collision boxes are the
  declared boxes rotated by the same matrix.
- Lookups for the hot path, each inline for the cube and air cases and a table
  read only for a shaped Voxel: `seals(voxel, face)`, `coverage(voxel, face)`,
  `quads(voxel)`, `collision(voxel)`, `volume(voxel)`, `light_mask(voxel)`
  (entry-blocked bits plus an "exit also blocked" bit for a shaped Voxel). An
  unregistered ShapeId is invisible and intangible: no coverage, no quads, no
  collision, volume 0.

`Voxel::is_full_cube`'s doc stops saying "conservatively transparent until the
second mesher pass exists".

## Tests

- The 24 Orientations are 24 distinct rotations, all of determinant +1, with
  byte 0 the identity; four turns return to the start; `from_bits` rejects 24
  and above.
- Canonical geometry: the slab seals only -Y; with `up` at -Y it seals only +Y;
  the stair seals -Y and +Z and nothing else; the post seals nothing; a cube
  seals all six.
- The stair's side coverage is an L, not a full square, so it does not seal -X
  or +X; a slab's side coverage is exactly one half.
- The stair's tall side lands at +Z, +X, -Z, -X for turns 0 to 3.
- Volumes: cube 4096, stair 3072, slab 2048, post 256, and they are the same
  under every Orientation.
- Internal faces are gone: the stair's quads sum to its exposed surface and no
  quad lies between two set sub-cells.
- Collision boxes rotate with the Orientation and stay inside the cell;
  `collision: Some(vec![])` gives none.
- Registering rejects each case above, hands out ids in order from 4, and
  `builtin()` is fast enough to build in a test.
