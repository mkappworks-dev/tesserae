# 14: The Client draws fixed Shapes

Status: done

Blocked by: None (can start immediately)

Spec: [Phase 16](../spec.md), amendment 2026-11-14.

## What to build

Found in play: the Workbench and Door previews in the game do not match the
mockup. The Client builds every Item's icon as a full cube (`item_holding` in
the Client's edit module) and draws the hand and the placement preview in the
Player's chosen Shape, ignoring a `voxel-uses` row's fixed Shape, while the
Server places the fixed Shape. Make the Client's content catalogue carry each
Material's fixed Shape, and use it everywhere an Item or held Material is drawn:
Inventory, hotbar, Catalogue and Crafting/Station icons, the hand, and the
placement preview. A Door's icon shows the panel's face, not its edge, as the
mockup's `/blocks` and recipe icons do; a Workbench shows the table. Compare
with `/blocks` and the recipe icons in `mockup/` (`python3 mockup/server.py`).

Also found in play, each for the Workbench as well as the Door and Gate: the
Workbench's icon is the table in the Crafting tab and wherever else it is
listed, and the Door and Gate icons in the Workbench (Station) pane show the
panel's face, as the mockup's `/station` does; they are Crafting icons, so the
same change covers them. The placement ghost shows the whole Door
or Gate as the Server will place it: every cell (a 1 m × 2 m Door, a 1 m × 1 m
Gate, at any Voxel size) in the panel Shape, turned to the facing and hinge side
the Server will choose; a Workbench's ghost is the table. The Client reuses the
Server's pure placement rule from `tesserae-core` (Door size, cells, orientation
and hinge) rather than duplicating it: it is a pure function of content and the
aim, so computing it on the Client keeps ADR-0002. A Dropped item of a
Workbench, Door or Gate draws in its fixed Shape, scaled down as a Dropped
cube is, not as a cube.

## Acceptance criteria

- [x] Icons for the Workbench, Door and Gate show the table and the panel face,
      matching the mockup.
- [x] Holding one, the hand and the placement preview show the fixed Shape
      whatever Shape the Player had selected.
- [x] Materials without a fixed Shape draw as before.
- [x] The Workbench's icon is the table wherever it is listed, the Crafting tab
      included, and the Workbench pane's Door and Gate icons show the panel's
      face, matching the mockup's `/station`.
- [x] The placement ghost of a Door or Gate shows every cell in the panel
      Shape, turned to the facing and hinge side the Server will place, at any
      Voxel size; a Workbench's ghost is the table. The rule is
      `tesserae-core`'s, shared with the Server, not a copy.
- [x] A Dropped Workbench, Door or Gate draws in its fixed Shape, scaled down
      as a Dropped cube is.
- [x] A Client test covers the icon and hand Shape, the ghost's cells and
      Orientation, and the Dropped item's Shape; review-only offscreen
      screenshots compare with the mockup, including the Workbench's icon in the
      Crafting tab, a Door ghost at 1 m and 0.25 m Voxels, a Gate ghost, a
      Workbench ghost, and a Dropped Door, Gate and Workbench.

## Decisions

- `CatalogueMaterial` carries its row's fixed `shape` and `door`, so the Client
  draws from content alone. `ContentCatalogue` is no longer `Eq`, as a Door's
  size is in `f32` metres.
- The aim answer's ghost is an `AimGhost`: the cell, plus for a Door the near
  face, hinge side and rise, which depend on the World. The Client applies
  `tesserae_core::placed_in` and `Door::footprint` to them; the Server places
  by the same two, and `door_faces`, `facing` and `Across::of` moved to
  `tesserae-core`.
- The hand and icons draw a fixed Shape unturned; the ghost turns a Workbench
  by the Player's Orientation, as the Server places it. The Shape key does
  nothing while a fixed Shape is held, so it cannot change the Shape chosen
  for other Items.
- Deferred: the ghost of a Door is vetted at its foot only, as before; a Door
  whose upper cells are blocked still shows and is refused on Place. A solid
  held outside the hotbar (a capture fixture, Layout) keeps the chosen Shape.
- Dev captures: `TESSERAE_SCREENSHOT_INVENTORY=station` opens the Workbench's
  pane, `TESSERAE_SCREENSHOT_HELD` takes a comma list, and
  `TESSERAE_SCREENSHOT_DROPS=<items>` shows resting drops ahead of the Eye.
