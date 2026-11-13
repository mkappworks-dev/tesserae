# 13: Doors are thin panels

Status: ready-for-agent

Blocked by: 09

Spec: [Phase 16](../spec.md), amendment 2026-11-14.

## What to build

A Door looks like a door, not a block. Placing a Door Item ignores the Player's
chosen Shape and Orientation: every cell gets a thin panel Shape (about 3/16 of
a Voxel thick) across the Player's facing, with the hinge side chosen from the
half of the cell aimed at. Use turns each cell's panel 90° about its hinge edge,
so an open Door lies along the hinge side of each cell: a Minecraft-style swing
at 1 m Voxels, turning slats at finer sizes (a whole-panel swing is later work).
Collision follows the panel, so an open Door no longer needs to pass bodies
through its Material: retire the open-Material exception (`Tables::blocks`) if
nothing else needs it. Rays, breaking, undo and redo work on the panel as on any
shaped Voxel; undo still matches either state. Ticket 12's Gate gets the same
panel. The look goes through the `/blocks` gallery (ticket 11) for the user's
approval.

The mechanism is general: any `voxel-uses` row (Door, Station or Container) may
declare a fixed Shape that placing its Material always uses, so a Workbench can
later be table-shaped and a Chest an inset box (the user is choosing those looks
in `/blocks`). Terra's Door uses it for the panel; Terra's Workbench and Chest
stay cubes in this ticket.

## Acceptance criteria

- [ ] A `voxel-uses` row of any kind may declare a fixed Shape, in WIT and
      data-only content; placing that Material ignores the Player's Shape and
      uses it.

- [ ] A placed Door is a thin panel in every cell, whatever Shape the Player had
      selected.
- [ ] Open, the panel lies on the hinge side and a Player walks through without
      any collision exception.
- [ ] Hinge side follows the aimed half; toggling twice restores the exact
      Voxels.
- [ ] Breaking, undo and redo still treat the Door whole and match either state.
- [ ] ADR-0026, GLOSSARY Door and the authoring guide describe the thin panel
      and the turn.
