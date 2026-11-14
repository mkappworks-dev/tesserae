# 13: The Client draws flat Plants

Status: ready-for-agent

Blocked by: 01, 03, 05

Spec: [Phase 18](../spec.md).

## What to build

What the approved mockup shows. An Item of a `cross` or `plane` Shape, and any
Item with a `texture`, is a flat sprite of its texture in the Inventory, hotbar,
Catalogue and Crafting icons, the hand, and as a Dropped item (upright and
turning at the Item's declared size). The placement ghost is the Shape at the
target, with each cell of a Layout of Seeds. The Use prompt reads Harvest on a
ripe crop. Everything else is unchanged.

## Acceptance criteria

- [ ] Matches the mockup for each surface, in review-only offscreen screenshots.
- [ ] A produce Item with a `texture` is flat; one without is the cube of its
      `color`.
- [ ] A solid Material's Items are unchanged.
- [ ] The Client reads no World storage; Stage changes arrive as ordinary Chunk
      edits.
- [ ] Client tests driven by key presses cover icon, hand, ghost, dropped sprite
      and the prompt.
