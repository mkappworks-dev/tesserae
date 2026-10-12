# The joint rule in `light_passes` and the incremental solver

Status: done
Blocked by: 01

## What to build

`light_passes(shapes, from, into, face)` blocks when the old rule does, or when
both Voxels are partial on the facing faces and their coverage is jointly full.
`brighten` already routes through it. `darken` keeps traversing every edge; its
comment gains one sentence on why the joint rule needs nothing there.

## Tests

- **Flip** `two_half_walls_that_only_jointly_seal_a_face_leak_light`: the light
  at the far side of the pair is 0, and rename it for what it now says.
- Siblings, through `OneChunk::place` (edit by edit): interlocking stairs seal,
  partial beside a cube source stays open (a lantern next to a slab still
  lights it), partial beside air is open, two bottom slabs let the upper half
  through.
- Place one half, then the other, then remove each: every Voxel returns to its
  previous light exactly, in both orders.

Mutation checks: restore the old rule in `light_passes` (flipped test fails);
drop the cube exemption (lantern test fails); union with AND (siblings fail).
