# 09: One undo removes a whole Structure

Status: done

Blocked by: 08

Spec: [Phase 20](../spec.md).

## What to build

A placement is one undo group. The journal counts cells, 131,072 in all, with
its 128 groups, instead of 1024 entries. Undo and redo stay all or nothing with
their present checks.

## Acceptance criteria

- [x] One undo removes a whole Structure and one redo restores it.
- [x] A placement fills the budget by evicting the oldest groups, never the
      wrong ones, and a Template at the cap fits twice.
- [x] Single-cell Edits are as undoable as before, by the existing journal
      tests.
- [x] Undo is refused as it is now when a Container the Structure placed has
      since been filled.

## Comments

- The cap is `undo::RECORDED_LIMIT`, 131,072 changed Voxels across the 128
  groups, now `pub(super)`. A const assert ties it to
  `2 * MAX_TEMPLATE_CELLS`, so a Template at the cell cap, sampled at 1 m
  cells in a 1.0 m World, fits twice. Eviction was already oldest-first and
  stays so; only Voxels that change are counted.
- A placement larger than the cap is refused before landing, never landed
  without an undo group: "a placement is one undo group" stays true, and a
  Creative Player never meets the one Structure undo cannot take back. The
  judge pass counts the cells that would change and refuses with the new
  `RejectReason::TooBigToUndo` ("That Structure is too big to undo at this
  Voxel size.") as soon as the count passes the cap, before any cell lands.
  This happens at 0.25 m (a 32 m hut is 139,008 Voxels), or with cells
  coarser than 1 m. If the World changes between judging and landing so that
  more cells change than were judged, the journal keeps the group alone, as
  it always has for an oversized group.
- `undo::restore` now writes a group in one `Batch` with one relight, rolled
  back in the same batch on a midway failure, instead of a relight per Voxel.
  An undo or redo of a whole Structure still lands in one tick.
- An undo or redo of a group over 1,024 Voxels still lands in one tick, so
  it keeps its all-or-nothing check. Its Edit events past `MAX_EDIT_EVENTS`
  in that tick wait in `EditHistory` in order and are released 1,024 a tick
  in `dispatch`, so a Mod called every tick hears every Voxel (review fix;
  `every_voxel_of_an_undo_past_one_ticks_events_reaches_a_mod_in_order` in
  `server/mods/runtime/tests.rs`).
- Tests: `server/edit/structure/tests/undo.rs` (one undo and one redo, the
  Container check, the oversized refusal), and
  `a_template_at_the_cap_fits_twice_and_the_oldest_groups_go_first` beside
  the existing journal tests in `server/undo.rs`, whose cell-bound test now
  uses quarter-cap groups.
