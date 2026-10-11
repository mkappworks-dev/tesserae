# The Server's edit path

Status: done
Blocked by: 03, 04, 05

What finally gives `relight` a real caller, and turns the step-4 `dev`
scaffolding into the thing it was a model for. Spec decision 7.

## What to build

`server/edit.rs`:

- `EditIntent { action: Break | Place(Voxel), origin: Vec3, direction: Vec3 }`
  and `EditRejected { reason }` as `Message`s, `add_message`'d by the Server
  plugin.
- A Server system in `RunFixedMainLoopSystems::BeforeFixedMainLoop` that drains
  `EditIntent`s into a Server-private `PendingEdits` every frame, so a Client
  write in a frame followed by one with no tick is never lost. The tick system
  applies up to `EDITS_PER_TICK` of them, ordered before `stream`.
- `apply`: ray via `collide::raycast` up to `EDIT_REACH_M` (metres); validate
  (registered Shape, valid Orientation, non-air Material, hit within reach,
  Place target is air, Chunk loaded, shaped cap not exceeded - net zero when
  replacing shaped with shaped); `set_voxel`; `relight` through `LightLattice`;
  mark dirty the edited Chunk, every Chunk the relight wrote, and the neighbour
  across any boundary plane the Voxel sits on.
- An edit whose Chunk is `unlit` is deferred a tick at a time and dropped after
  64. An edit into an unloaded Chunk is rejected.
- A `Shapes(ShapeTable)` resource, `ShapeTable::builtin()`, in `server/mod.rs`,
  and `streaming::publish` and `relight` pass it on.
- `impl VoxelVolume for ChunkStore`, read-only.
- The shared Server-internal entry the dev builders use: one function that
  applies a known Voxel at a known position through the same cap, relight and
  dirtying, so there is one edit path. The lightbox calls it.
- Delete every `cfg_attr(not(feature = "dev"), allow(dead_code))` that now has a
  non-`dev` caller. Any that remain say why in the PR.
- `MeshState::edited`'s comment, `ChunkStore::set_voxel`'s and
  `world_state.rs`'s block comment stop saying "build-order step 5 brings a real
  edit path".

## Tests

- Break then Place through the message path, in a headless `App`: the Voxel
  changes, the light follows, and a `ChunkMeshReady` is sent for the edited
  Chunk.
- **An edit on a Chunk boundary re-meshes the neighbour even when no light
  changes**: remove a stone Voxel in the dark and assert both Chunks are
  redelivered.
- A message written in a frame that runs no tick is applied by a later one.
- Rejections: unregistered Shape, orientation 24, air Material, out of reach,
  occupied target, unloaded Chunk, shaped cap (and replacing a shaped Voxel at
  the cap succeeds).
- A deferred edit on an `unlit` Chunk applies once the Chunk is lit and is
  dropped after 64 ticks.
- The existing lightbox tests still pass and the lightbox now goes through the
  shared entry.
- A mesh produced by an edit is the mesh `mesh_chunk` would produce for the same
  store (no stale face).

## Comments

The `Shapes` resource landed in ticket 02, where `publish` first needed it.

The cfg attributes: every `cfg_attr(not(feature = "dev"), allow(dead_code))` on
`ChunkStore`, `VoxelCoord`, `LightLattice`, `set_voxel`, `light_at` and
`MeshState` came off, and the build is warning-free without `--features dev`.
`WorldSettings::voxel_at` keeps its, because converting a metre position to a
Voxel is still done only by the dev builders; the edit path gets its Voxel from
the ray.

Landed in one commit with ticket 07: `EditAction` and `EditRejected` are dead
code until the Client constructs and reads them, so neither passes `-D warnings`
without the other.

A cube is stored with its orientation dropped (a turn on a cube is validated and
then discarded), so two cubes that look identical are one palette entry.

Mutation check: removing the boundary-neighbour marking fails both the unit test
and the streaming end-to-end test.

One test needed care: `run_system_once` builds a fresh `MessageReader` on every
call, so a helper that delivers one intent at a time re-reads every earlier one.
The helper empties the buffer after each delivery; the real system keeps its
reader across frames.
