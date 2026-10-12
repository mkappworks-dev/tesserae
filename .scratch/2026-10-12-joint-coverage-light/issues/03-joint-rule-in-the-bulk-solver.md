# The joint rule in `light_chunk`, `flood` and `seed_from_border`

Status: done
Blocked by: 01, 02

## What to build

Per the spec's "How it stays off the hot path":

- A per-cell `partial` array built in `light_chunk`'s existing sweep, **only if**
  `center.may_contain_shaped()`; empty otherwise.
- `flood::<true, CHANNEL>` takes the Chunk and the table, hoists the source's
  partial bits, and on the cold branch asks `light_passes`. `flood::<false, _>`
  is not edited.
- `seed_from_border` applies the rule across the plane, reading the inside
  Voxel only when the inside cell is partial on that face and the neighbour may
  hold a shaped Voxel.

## Tests

- The agreement tests gain: bottom beside top slab, interlocking stairs, inside
  one Chunk and on the seam (a half on each side, and both halves on one side),
  sky and block channels.
- The pinned scene `two half walls` in
  `the_two_solvers_agree_on_shaped_scenes_inside_one_chunk` now expects the
  sealed answer from both solvers.
- Bulk and incremental agree at a fixed point on a scene with a roof of
  alternating slabs over open ground (the sky channel).

Mutation checks, each of which must fail a named test: drop the joint test in
the flood's cold branch; drop it in `seed_from_border` (west side, then east
side of the seam); gate the cold branch on the source's bits only; build the
`partial` array only for the first half of the Chunk.

## Performance

Run the Performance list of the spec against the `before` baseline.
