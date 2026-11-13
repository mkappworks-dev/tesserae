# 13: Doors are thin panels

Status: done

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

- [x] A `voxel-uses` row of any kind may declare a fixed Shape, in WIT and
      data-only content; placing that Material ignores the Player's Shape and
      uses it.

- [x] A placed Door is a thin panel in every cell, whatever Shape the Player had
      selected.
- [x] Open, the panel lies on the hinge side and a Player walks through without
      any collision exception.
- [x] Hinge side follows the aimed half; toggling twice restores the exact
      Voxels.
- [x] Breaking, undo and redo still treat the Door whole and match either state.
- [x] ADR-0026, GLOSSARY Door and the authoring guide describe the thin panel
      and the turn.

## Decisions

- `voxel-use` gains `shape: option<string>` (data-only `shape = "..."`), any
  kind. A Station or Container keeps the Player's Orientation; a Door's Shape
  is drawn closed against its own +Z and Terra declares it as `terra:panel`.
- The hinge side lives in the Orientation: a Door hinged the other way is
  placed upside down, so Use always turns `turned()` one way, two toggles
  restore the exact Voxels, and the closed panel stands on the near face
  whichever side the hinge is.
- The pass-through rule is narrowed, not retired: an open Door collides as
  drawn when it has a fixed Shape and is one Voxel wide (Terra's at 1 m).
  At finer sizes the turned slats stand about 0.2 m apart, so an open Door
  still lets bodies through there, keeping ticket 09's walk-through at
  0.25 m; a Door with no fixed Shape does too.
- The Door's two states share one texture; the open one is no longer a
  cut-out frame. The Terra texture pin and Shape count moved, and the
  distinct-artwork test allows the Door pair.
- Terra's panel Shape is placeable by the Player in any Material, as a sixth
  Shape chip.
- Deferred: the Client's ghost still previews one Voxel in the chosen Shape,
  and Layouts and Mod Commands of a fixed-Shape Material place what they are
  given. The Gate (ticket 12) uses the same `shape` row when it lands.
