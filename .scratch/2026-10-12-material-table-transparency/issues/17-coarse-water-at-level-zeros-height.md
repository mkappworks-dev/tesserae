# A coarse water surface sits at level 0's height

Status: ready-for-agent
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
