# `FaceMask::jointly_full` and `ShapeTable::light_partial`

Status: done

Spec: [joint coverage in the light edge rule](../spec.md). Nothing depends on
light.rs yet.

## What to build

- `FaceMask::jointly_full(&self, other: &FaceMask) -> bool`: every row of
  `self | other` is all ones; early out on the first row that is not.
- A `partial: bool` on `Variant`, true when some face's coverage is neither
  empty nor full, derived at registration beside `seals`.
- `ShapeTable::joint_id(voxel) -> u16` (variant slot + 1 if partial, else 0; cube,
  air and an unknown Shape are 0) and `ShapeTable::jointly_seals(from, face,
  into) -> bool` over two non-zero ids. `light_mask` is unchanged.

## Tests

- `jointly_full`: half planes in either order, two equal halves (false), a full
  mask with anything (true), empty with empty (false), the full-with-empty
  boundary.
- `joint_id` is non-zero for the built-in slab, stair and post and zero for a
  cube, air and an unknown Shape.
- `jointly_seals`: bottom slab with top slab (both ways round, on the sides it
  faces), two bottom slabs (no), a stair with its mirror image (yes) and with a
  copy of itself (no).
- Invariant over all built-ins and all 24 Orientations: each face is exactly one
  of sealed, partial or empty, and `joint_id != 0` iff some face is partial.

Mutation checks: make `jointly_full` an AND, then drop the early-out's guard;
make `partial` count a sealed face as partial.
