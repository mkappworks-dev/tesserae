# 03: Use on Doors

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Phase 16](../spec.md).

## What to build

A player aims at a Terra door, sees "G Open" under the reticle, presses G and
walks through. Line 0.3 gains `voxel-uses` rows with the `door` kind (a closed
and an open solid Material); data-only content gets `[[voxel-uses]]`. Use no
longer requires a Tool: a held Tool runs, otherwise a Usable Voxel in Edit reach
is used, otherwise the selected Stack is eaten. The aim answer's `use_aim`
carries the kind and verb, or the out-of-reach refusal, and the Client shows the
prompt as the mockup does. Use on a Door toggles the vertical run of connected
Voxels of that Door, at most 4, as one Edit group that never enters the undo
journal, keeping Shape and Orientation. Closing into the Player's body is
refused with Place's refusal. Removing the open Material drops the closed
Material's Item. Undo and redo match either Door state. The "Use tool" binding
is renamed "Use". Terra declares a wooden Door.

## Acceptance criteria

- [ ] Use priority holds: Tool, then Usable Voxel, then eating; eating with
      nothing usable aimed at still works.
- [ ] A one-Voxel and a two-Voxel Door open and close as one; a run longer than
      4 toggles 4.
- [ ] Shape and Orientation survive a toggle; a toggle adds no journal entry.
- [ ] Closing on the Player is refused; out of reach is shown before the press
      and refused after it.
- [ ] Breaking an open Door drops the closed Material's Item; undo of a Door's
      placement works open or closed.
- [ ] A Material declared as two kinds, or in two Doors, is a load error.
- [ ] An open Door survives reopening the World.
