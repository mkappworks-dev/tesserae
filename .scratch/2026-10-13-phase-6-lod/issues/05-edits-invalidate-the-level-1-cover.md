# Edits invalidate the level-1 cover

Status: done
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

Built as specified except where listed. 7 tests in `server/lod.rs`; nothing in
the level-0 suite changed. Mutation checks, each restored: dropping the
`cover_changed` insert in level 0's `retire` fails the edited-eviction test;
marking on every eviction, edited or not, fails the unedited-eviction test
(a held cover is rebuilt); clearing `stale` at landing fails both flight tests.

Where the ticket was wrong or silent:

- **`edited` is not the hook; `voxels_changed` is.** `set_voxel` calls `edited`
  for the edited Chunk, for every Chunk the relight wrote light to, and for the
  neighbour across a boundary plane. A cover is derived from Voxels only, so
  marking those would rebuild a cover for a lantern's light. `set_voxel` gains
  one line, `meshes.voxels_changed(place.chunk)`, beside the `edited` call;
  signature unchanged, and the `dev` builders and gallery go through it.
- **Plumbing.** `MeshState` cannot see `LodState`, and should not. It keeps
  `edited_chunks` (the set the ticket asks for) and `cover_changed` (level-0
  coords, filled by an Edit and by `retire` when an edited Chunk leaves).
  `stream_lod` drains `cover_changed` each tick, maps to `floor(coord / 2)` and
  calls `invalidate(1, cover)`. `stream` runs first in the chain, so an
  eviction and the regeneration it causes are in the same tick. With one level
  `stream_lod` drains and discards, or the set would grow for good.
- **"Held" includes "being generated".** An Edit while the cover's first
  generation is in flight would otherwise be missed: the job took its children
  at dispatch, lands without the Edit, and nothing marks it. The drain marks a
  cover that is held or has a generate job; a cover that is neither will read
  the edited Chunk when it is made. Its own test. Dropping the guard changes
  nothing observable (dispatch clears the mark of an absent cover in the same
  tick), so no test pins it; it only keeps `stale` free of coords nothing holds.
- **The regeneration-lands-dirty-neighbours item** was ticket 03's and is
  covered by `an_invalidated_chunk_is_regenerated_and_it_and_its_neighbours_
  are_meshed_again`; nothing was added. The new tests check the redelivered mesh equals a fresh
  `mesh_chunk` of the store.
- `invalidate` lost its `allow(dead_code)`. `MeshState::is_edited` is test-only.
  No `pub` mutator outside `crate::server`.

Tests: an Edit of a 2x2x2 block (one coarse Voxel's footprint) in an empty
Chunk, so the cover is sure to change. `quiet_chunk` picks one on the Eye's side
of the world whose cover is meshed and which leaves `evict` when the Eye walks
160 m along +x while its cover stays. "Dispatches nothing" is checked two ways:
generate jobs dispatched over the whole fill equal the Chunks held (nothing
built twice) when the cover was absent, and `Arc::ptr_eq` on every surviving
cover when an unedited Chunk leaves.
