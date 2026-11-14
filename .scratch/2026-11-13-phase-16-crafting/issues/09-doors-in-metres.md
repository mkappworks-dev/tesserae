# 09: Doors in metres

Status: done

Blocked by: None (can start immediately)

Spec: [Phase 16](../spec.md), amendment 2026-11-14.

## What to build

A Door is a size in metres, not a run of Voxels, so it is taller than the 1.8 m
Player body and wide enough to walk through at every Voxel size (ADR-0003). Line
0.3's `door` kind carries the open Material plus an optional width and height in
metres (1 m by 2 m when absent; validate a sane range, e.g. width 0.5 to 4 m,
height 1 to 4 m), in WIT and in data-only `[[voxel-uses]]`. Placing one Door
Item places the whole Door: ⌈width ÷ Voxel⌉ by ⌈height ÷ Voxel⌉ cells, one Voxel
thick, across the Player's facing, resting on the struck face, spending one
Item, as one undo group, refused (with the existing Place refusals) if any cell
is blocked, unloaded or inside the Player. Use toggles every cell of that Door
together. Breaking any cell removes the whole Door and drops one Item; undo of
that removal restores it whole. Two Doors placed side by side toggle and break
independently: one way is to take the connected cells of that Door Material in
its plane and split them into Door-sized tiles from the component's lower
corner, but choose what works and record it. Retire `DOOR_RUN_MAX`. Terra's Door
declares 1 m by 2 m.

## Acceptance criteria

- [x] At 1 m and at 0.25 m Voxels, one Door Item places a Door at least 2 m tall
      and 1 m wide, and a Survival Player walks through it open.
- [x] Placement is refused whole when any cell is blocked, and spends exactly
      one Item.
- [x] Use toggles every cell; breaking any cell removes all and drops one Item;
      undo and redo restore or remove the whole Door in either state.
- [x] Two Doors side by side toggle and break independently.
- [x] An out-of-range size is a load error; an absent size is 1 m by 2 m.
- [x] Docs (authoring guide, data-mods, contract-compatibility, ADR-0026,
      GLOSSARY Door) say a Door is a size in metres.

## Decisions

- Which cells are one Door is derived, not stored: the cells of that Door's
  Materials touching in its plane are tiled Door-wide from their lowest corner,
  then each column of tiles Door-tall from its own lowest cell. The plane is the
  horizontal axis the Material runs further along through the aimed cell. Doors
  side by side, even on a step, stay two; Doors stacked out of line are not told
  apart. Recorded in the ADR-0026 amendment.
- A Door placed against a ceiling (solid above its foot, none below) hangs
  down; otherwise it stands up from its foot, centred across the facing.
- Deferred: the Client's ghost still previews one Voxel, and Layouts and Mod
  Commands of a Door's Material still place single Voxels.
