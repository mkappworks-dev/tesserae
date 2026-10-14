# A coarse water surface sits at level 0's height

Status: done
Blocked by: 07, 13

A coarse water Voxel is water when water holds at least half of it, so a
coarse lake's surface is the top of a coarse Voxel: up to half of one away from
the real surface, either way. Where a finer level's row boundary falls in that
gap, ticket 07's cut disagrees with what the finer level drew. A surface pushed
into an unsent Chunk above the finer rows is not cut and doubles the finer one
(ticket 14), and a surface pushed into a sent Chunk below them is cut, though
the finer level has no water there, which leaves a hole. Neither can happen at
the default water level, whose 44 m is no level's row boundary, but a Mod's
level or another Voxel size can put one there.

## What to build

- The Server moves the top face of every coarse water mesh to the height of
  level 0's own surface, the top of its highest water Voxel. Every level then
  draws its surface where level 0 does, and the cut asks about the Chunk the
  finer surface is in.
- The cut samples the Chunk behind a face, a hair along its normal into the
  Voxel it belongs to, so a surface lying exactly on a Chunk boundary is asked
  about the Chunk whose mesh holds it.

## Acceptance criteria

- Test: a coarse water Chunk whose water rounds up and one whose water rounds
  down both have their top faces at level 0's surface height, at more than one
  level; side faces and other Materials are untouched; a dry World's mesh is
  unchanged.
- Capture of a ring join over a lake shows no step in the water surface between
  levels, and the magenta probe finds no hole.
- Ticket 14 closes as fixed by this, and the mirror case at a level's bottom
  edge is recorded on ticket 07.
- `./scripts/cargo-ci` passes.

## Comments

Built as specified. `level_the_water` in `server/lod.rs` runs at the end of
`mesh_coarse`: every upward Water quad within one coarse Voxel of
`ceil(level_m / step_m) * step_m` - level 0's own surface, the top of its
highest water Voxel - has its corners moved to that height. Within one coarse
Voxel, because a Water face further away belongs to another body of water, not
to this surface. Only the top faces: a side quad is split by row, each row's
underwater light being its own, and moving one would tear the column.

The cut now samples a hair behind the face. `finer_drawn` takes the fragment's
world normal and asks about `point - normal * cell_m / 1024`, a thirty-second
of a finer Voxel, because a levelled surface can lie exactly on a Chunk
boundary and `floor` would then name the Chunk in front of the face - the one
above it, which holds no water. `drawn_mask::contains`, the CPU mirror the
tests read the buffer with, carries the same constant.

- Tests: a coarse water Chunk whose water rounds up (12 rows of 4 m, to 48 m)
  and one whose water rounds down (11 rows, to 44 m) both draw their top faces
  at 45 m, level 0's surface under water at 44.5 m, at levels 2 and 3; the
  bottom faces stay at 0, the highest side quad still reaches the water's own
  top and no side left its row boundary; and a dry World's mesh is untouched.
  In `chunk_view`, an upward face exactly on a Chunk boundary is cut by the
  Chunk below it, which was sent, and a downward face at the same point is not,
  the Chunk above it having been sent nothing.
- Capture (offscreen, 1080p, water at 45 m, 60 m up at 12 degrees down over a
  lake that crosses the level-0/1 and 1/2 joins, 30 s settle, no flight so the
  picture is deterministic - two runs of the same build differ in 0 pixels).
  Before: a straight horizontal edge runs across the lake where the coarse
  surface stands a Voxel above level 0's, with its lit side showing. After: it
  is gone and the lake reads as one surface. 40034 pixels change, all of them
  between rows 243 and 470, which is the distance; the water loses 6514 pixels,
  the shore the lower surface uncovers. 45 m is the level to see it at: at the
  default 44 m only levels 3 and up round, and they are more than 1.5 km out.
- Magenta probe, same view and the horizon view: 3 clear pixels in one column,
  where the terrain's top edge steps 11 pixels between neighbouring columns -
  a silhouette step, the same count ticket 07 reported, and no hole. The
  horizon view gains a fourth pixel at that same step, the water's edge having
  moved down a pixel.
- With the flight on, the capture is not deterministic (3.6% of pixels differ
  between two runs of one build at the horizon view), so the comparison above
  uses `TESSERAE_SCREENSHOT_FLY_S=0`. Worth knowing before any future capture
  is diffed pixel for pixel.
- Ticket 14 is `done`, fixed by this. The mirror case at the bottom edge of a
  level's rows, which this does not fix, is recorded on ticket 07.
