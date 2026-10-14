# Coarse water a Voxel above the finer level's top row

Status: wontfix
Blocked by: 07

Ticket 07 cuts a coarse blended fragment inside a finer Chunk the Client was
sent. A coarse water surface rounded up past the finer level's top row lies in
a finer Chunk that was never sent, so it is not cut and can double the finer
surface below it.

Not fixed, deliberately. Cutting a coarse Voxel's height above the finer rows
too would also cut a real water surface there, which only the coarse level
draws: the finer level's top-row water culls its face against the water in the
Chunk above, which it holds but does not mesh. That turns a rare doubled band
at the vertical edge of a level into a rare hole. The finer level would need to
mesh its boundary faces to make the cut safe.

## Comments
