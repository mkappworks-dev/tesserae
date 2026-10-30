# 02: Server layout intent, caps and grouped undo

**What to build:** one batched layout intent the Server validates and applies.
The Server resolves the anchor from the aim ray, expands the geometry itself
(ticket 01), skips cells that fail the single-Place rules, and answers aim
reports with the accepted, unreachable and blocked cells for the preview.
Applied cells land as one undo group.

**Blocked by:** 01

**Status:** ready-for-agent

Spec: [layout placement](../spec.md).

- [ ] A layout is one pending item. It drains through `EDITS_PER_TICK` and
      cannot exceed 256 Voxels or starve other intents.
- [ ] Each cell is judged within `EDIT_REACH_M` of the Eye, empty, loaded and
      lit; a failing cell is skipped and the rest land.
- [ ] The Client sends no cell list; the Server expands the geometry.
- [ ] One undo and one redo cover the whole layout atomically; any changed
      cell refuses the whole undo with `HistoryChanged` and changes nothing.
- [ ] The journal bound on total recorded Voxels is fixed and tested.
- [ ] Mods receive the same Edit events as for single Edits; no WIT change.
- [ ] The architecture.md undo paragraph is updated to describe groups.
- [ ] Single-Voxel Place, Remove and history behave exactly as before.
