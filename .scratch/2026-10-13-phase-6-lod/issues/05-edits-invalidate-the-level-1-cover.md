# Edits invalidate the level-1 cover

Status: ready-for-agent
Blocked by: 04

Spec decision 6. Server only; the Client is not involved.

## What to build

- `MeshState::edited(coord)` also records the Chunk as edited and calls into
  `LodState`: if the level-1 cover (`floor(coord / 2)` per axis) is held, mark
  it `stale`; otherwise nothing.
- When a level-0 Chunk that has been edited leaves the store (`retire`), mark
  its level-1 cover `stale` and drop it from the edited set. An unedited Chunk
  leaves without any mark.
- A regenerated cover that lands marks held neighbours `dirty` (ticket 03) so
  the new mesh is built.
- Levels 2 and above are never invalidated; a comment says why (an Edit is
  within `EDIT_REACH_M` of the Eye, level 2 starts at 256 m, level-0 eviction is
  at 288 m).

## Tests

- Edit a Voxel in a held level-0 Chunk whose level-1 cover is held: at
  quiescence the cover equals `downsample` of its children with the edit, and
  its mesh was redelivered.
- The same Edit when the cover is not held: no job is dispatched, and when the
  cover is generated later it contains the edit.
- Evicting an edited Chunk regenerates its cover without the edit; evicting an
  unedited Chunk dispatches nothing.
- An Edit during a cover's generation flight leaves it `stale` and a fresh job
  follows (the "mark during flight survives landing" case, its own test).
- Sustained Edits still deliver meshes (starvation).

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- No new `pub` mutator outside `crate::server`.

## Comments
