# Coarse water a Voxel above the finer level's top row

Status: done
Blocked by: 07

Ticket 07 cuts a coarse blended fragment inside a finer Chunk the Client was
sent. A coarse water surface rounded up past the finer level's top row lies in
a finer Chunk that was never sent, so it is not cut and can double the finer
surface below it.

Fixed by ticket 17, not the way below. Not fixed at the time, deliberately: Cutting a coarse Voxel's height above the finer rows
too would also cut a real water surface there, which only the coarse level
draws: the finer level's top-row water culls its face against the water in the
Chunk above, which it holds but does not mesh. That turns a rare doubled band
at the vertical edge of a level into a rare hole. The finer level would need to
mesh its boundary faces to make the cut safe.

## Comments

2026-10-14: closed by ticket 17. A coarse water mesh's top face is moved to
level 0's own surface height, so a coarse surface no longer rounds up past the
finer level's top row and the case this ticket describes cannot arise. The
rejected fix above - cutting a coarse Voxel's height above the finer rows - is
still the wrong one, and the finer level still does not mesh its boundary
faces; neither is needed now that every level draws the surface at one height.
