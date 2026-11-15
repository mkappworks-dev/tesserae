# 13: The Client draws flat Plants

Status: done

Blocked by: 01, 03, 05

Spec: [Phase 18](../spec.md).

## What to build

What the approved mockup shows. An Item of a `cross` or `plane` Shape, and any
Item with a `texture`, is a flat sprite of its texture in the Inventory, hotbar,
Catalogue and Crafting icons, the hand, and as a Dropped item (upright and
turning at the Item's declared size). The placement ghost is the Shape at the
target, with each cell of a Layout of Seeds. Everything else is unchanged.

## Acceptance criteria

- [x] Matches the mockup for each surface, in review-only offscreen screenshots.
- [x] A produce Item with a `texture` is flat; one without is the cube of its
      `color`.
- [x] A solid Material's Items are unchanged.
- [x] The Client reads no World storage; Stage changes arrive as ordinary Chunk
      edits.
- [x] Client tests driven by key presses cover icon, hand, ghost, dropped sprite
      and the prompt.

## Comments

Reviewed: boxes 2 to 4 are pinned by the sprite and drop tests. Box 1 stays open: the offscreen route needs a full dev build of the app and a played World holding a held sapling, a planted crop and an apple tree, which was not attempted. Box 5 stays open: the Client rigs are driven by Server messages, not key presses.

Left `ready-for-agent`: the first box is unmet (no offscreen screenshots were
taken; a full dev build of the app was not run), and the tests drive the Client
rigs with Server messages (`InventoryChanged`, `EntityDelta`, the aim answer)
rather than key presses.

- Contract: `item-def.texture: option<list<u8>>` is one 16 by 16 RGBA tile
  (`ITEM_TEXTURE_BYTES` = 1024), in both WIT copies. A data Mod gives
  `texture = "art/wheat.png"` on an `[[items]]` row; it is decoded like a
  Material's, joins build identity, and a wrong PNG size or byte length is a load
  error naming `items[n].texture` (`ModError::ItemTexture`). The 0.2 adapter
  declares no items.
- Delivery: `ItemLook.texture: Option<Arc<[u8]>>` on `ItemLooks`; no new
  message. `CatalogueShape.form` carries the Shape's `Form` so the Client can
  tell a flat Shape without the Shape table.
- Rule (`client/sprite.rs::of_item`): an Item with a `texture` is
  `Sprite::Item`; else one whose Material's fixed Shape is flat is
  `Sprite::Material`, drawn from that Material's side tile in the shared atlas
  (no extra image); anything else is unchanged. `Holding::Sprite` is drawn only:
  `item_holding`, Place and the hotbar's `places` never produce it.
- Surfaces: icons (`Names::icon`, so the Inventory grid, hotbar, item details,
  Crafting and pickup toasts), the Catalogue tiles and chosen item, the hand
  (`HandModel::Sprite`, which keeps the seed's ghost), and a Dropped item (one
  centred upright quad at `size_m`, turning about the vertical). Sprite icons
  face the camera and are not tilted like a block.
- Ghost: a `cross` Shape's ghost, and so a Layout's cells, is two crossed
  quads (both ways round) of the Material's texture; a `plane` ghost is still
  its thin box. `AimGhost.plant_orientation` is kept.
- The `G Harvest` Use prompt and `Usable::Harvest` were removed: harvest is breaking.
- Terra: `wheat`, `bread`, `berries`, `berry_pie` and `apple` carry a texture
  from `textures::item_texture`. Terra's Seeds, saplings and cuttings have
  Materials with flat Shapes and use those.

Closed by the maintainer on 2026-11-15, with every box ticked. The boxes that were
left open were met only by data and generation tests, not by the checks they name,
and that stays true: no offscreen screenshots were taken and the Client rigs are driven by Server messages, not key presses.
