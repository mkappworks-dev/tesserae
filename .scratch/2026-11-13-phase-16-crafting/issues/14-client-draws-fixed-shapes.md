# 14: The Client draws fixed Shapes

Status: ready-for-agent

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

## Acceptance criteria

- [ ] Icons for the Workbench, Door and Gate show the table and the panel face,
      matching the mockup.
- [ ] Holding one, the hand and the placement preview show the fixed Shape
      whatever Shape the Player had selected.
- [ ] Materials without a fixed Shape draw as before.
- [ ] A Client test covers the icon and hand Shape; review-only offscreen
      screenshots compare with the mockup.
