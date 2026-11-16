# 08: The Server places a Structure for a Creative Player

Status: done

Blocked by: 02

Spec: [Phase 20](../spec.md).

## What to build

A Structure intent names a Template, an anchor from the aim ray and a quarter
turn. The Server expands the cells, judges them as a whole and lands them
through the one World change in per-tick batches with one relight per batch.
Only the anchor must be in reach. It is Creative only and spends no Item. It
waits a bounded number of ticks for unloaded or unlit Chunks, then refuses with
a reason, and it lands whole or not at all. Mods hear the Voxels as Edit events.

## Acceptance criteria

- [x] A Structure larger than the edit reach lands whole when its anchor is in
      reach.
- [x] Survival refuses with a reason, and Creative spends no Item.
- [x] A placement over an unloaded Chunk waits, then lands whole or refuses
      after the bound.
- [x] A refusal leaves the World unchanged.
- [x] A placed Structure is saved like any build and reloads.
- [x] The Edit events reach a Mod.
- [x] A Structure at 0.25 m lands without blocking the tick beyond the existing
      per-tick budget.

## Comments

Where things live, for 09 and 10:

- `crates/tesserae/src/server/edit/structure.rs`. The intent is
  `edit::StructureIntent { origin, direction, template: String, turn: Turn }`,
  a player intent registered in `edit::plugin`; `structure::receive` queues it
  and `structure::drain` runs in `FixedPreUpdate` right after `apply_pending`.
  The answer to the intent is the World change, or an `EditRejected`.
- `Placement::new(template, voxel_size, turn, anchor)` expands the cells:
  `len()` and `get(index) -> Option<(at, Voxel)>`, `None` for Keep, air for a
  carving cell, x fastest, then z, then y from the bottom. The footprint is
  centred on the anchor across (lowest corner at `anchor - size / 2`) and the
  Ground layer's lowest Voxel layer is level with the anchor, so cellar layers
  go into the ground aimed at.
- `structure_anchor(store, tables, world, direction, reach)` is the cell in
  front of the face the ray strikes, as a single Place picks it.
- `judge_cell(store, meshes, tables, world, (body, containers), at, voxel)`
  gives a `StructureCell`: `Places`, `Replaces`, `Carves`, `Unchanged` or
  `Refused(reason)`. 10's aim answer can tint the ghost from it per cell.
- New `RejectReason::UnknownTemplate`. Survival is `CreativeOnly`; nothing is
  ever spent.

Decisions:

- A non-Keep cell overwrites whatever is there, as worldgen's does. A cell
  refuses the whole placement when its Chunk is missing or unlit (waited for),
  when it would overwrite a Container holding Stacks, when a solid cell would
  land in the Player's body, or when a Chunk's shaped cap would be passed.
- Every cell is judged before any lands, then the cells land in index order.
  The per-tick budget is the Mod drain's (`MOD_VOXELS_PER_TICK`: 4096 looks
  and 96 changed Voxels, an emitter counting as `EMITTER_WRITES`), made
  `pub(in crate::server)` and reused: the same batched writer, one relight per
  tick's batch through `Batch`. It also keeps the Edit events a tick adds well
  under `EditHistory`'s 1024, so a Mod hears every Voxel.
- The wait is `MAX_DEFERRED_TICKS` (64) ticks over the whole placement, not
  per cell. A refusal while landing (a Chunk turning unlit past the bound, the
  Player stepping into a cell) takes back what had landed in one batch, and
  Mods hear the inverse events, as `land_whole` does.
- Only cells that change are journalled and heard. A placement is one
  `Journal::record_group`; the journal still caps at 1024 entries and keeps
  the newest group whole, so a large Structure evicts every older group. 09
  owns the cell budget, and with it what to do with a placement over 131,072
  changed cells: at 0.25 m one Template can reach millions of Voxels.
- `land` is split into `land_in`, which writes into a caller's `Batch` and
  returns the entry, and `land`, which batches one write and records it. The
  cube/connected-Shape normalisation is `stored_solid`.
- The Tool places no foundation fill; worldgen's foundation is 03's. No Sound
  plays for a placement yet, and a Door half overwritten is left as it is.
- At most 4 placements queue at once (`Busy` past that), front first; a
  waiting one holds those behind it.
- The persistence tests need `./scripts/build-guests` run in a fresh worktree
  (`games/terra/dist`), or every Rig test fails looking up `terra:*`.
