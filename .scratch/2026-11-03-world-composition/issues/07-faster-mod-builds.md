# 07: A Mod's build should appear near the Player first and not take minutes

Status: needs-info

## What to build

A Game that builds a structure on start (`lumen`'s garden, `meadow`'s glade, `ember`'s
chimneys) waits on the Server's Mod drain: two Voxels a tick, in the order the Game
sent them, with no regard for where the Player is. ADR-0012 records the two as the
measured budget, because each write re-floods light and marks meshes stale. The demo
Games now send nearest-first and keep their builds small (lumen 12,616 Voxels down to
2,476), which hides the cost without removing it.

Options to measure and choose between, with benchmarks:

- A bulk write path that lights and meshes a region once instead of per Voxel, which would
  raise the budget by orders of magnitude.
- Draining the Voxel nearest the Player first across a Mod's queue, not strictly in order.
- Not spending budget on a write that changes nothing, such as air over air.

## Open question

Which of these is worth a new Engine capability, given the spec rule that a Game needing
one the contract lacks is a gap to record, not to work around.

## Acceptance criteria

- [ ] A benchmark shows Voxels applied per second for a plain fill, before and after.
- [ ] `lumen` at 0.5 m Voxels (19,712 Voxels) builds in under 10 s on the reference host.
- [ ] ADR-0012's "two Voxels per tick" consequence is updated with the new measured figure.
