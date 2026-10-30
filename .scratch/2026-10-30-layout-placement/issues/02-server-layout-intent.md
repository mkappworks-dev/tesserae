# 02: Server layout intent, caps and grouped undo

**What to build:** one batched layout intent the Server validates and applies.
The Server resolves the anchor from the aim ray, expands the geometry itself
(ticket 01), skips cells that fail the single-Place rules, and answers aim
reports with the accepted, unreachable and blocked cells for the preview.
Applied cells land as one undo group.

**Blocked by:** 01

**Status:** done

Spec: [layout placement](../spec.md).

- [x] A layout is one pending item. It drains through `EDITS_PER_TICK` and
      cannot exceed 256 Voxels or starve other intents.
- [x] Each cell is judged within `EDIT_REACH_M` of the Eye, empty, loaded and
      lit; a failing cell is skipped and the rest land.
- [x] The Client sends no cell list; the Server expands the geometry.
- [x] One undo and one redo cover the whole layout atomically; any changed
      cell refuses the whole undo with `HistoryChanged` and changes nothing.
- [x] The journal bound on total recorded Voxels is fixed and tested.
- [x] Mods receive the same Edit events as for single Edits; no WIT change.
- [x] The architecture.md undo paragraph is updated to describe groups.
- [x] Single-Voxel Place, Remove and history behave exactly as before.

## Comments

### 2026-10-30: implemented

Reach and sight are judged for the whole layout when it starts, before any
cell lands: asked per cell as it fills, a layout's earlier cells hid its far
cells from a low Eye. Each cell is then re-checked for occupancy when applied.
The journal holds at most 128 groups and 1024 recorded Voxels. Existing
single-Place rules do not test the Player's body, so layouts do not either.
