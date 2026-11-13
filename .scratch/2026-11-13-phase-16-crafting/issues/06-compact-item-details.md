# 06: Item details: compact, with Shape previews

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Phase 16](../spec.md).

## What to build

The Inventory window's item details panel matches the mockup: the icon beside
the name, drawn in the chosen Shape; Count and Max in one row (Shape leaves the
stats, since it is the Player's choice, not part of a Stack); the five Shapes as
chips that preview the Item in each Shape; the actions (To hotbar or To
backpack, Split, Drop 1, Drop stack) in a two-by-two grid, with Cancel move on
its own row while moving. The panel never scrolls at the default window size.

## Acceptance criteria

- [ ] The panel fits without scrolling at the default window size and at 100%
      text.
- [ ] Each Shape chip previews the Item in that Shape and selecting it changes
      the header icon and the Player's Shape choice.
- [ ] The actions behave as before.
- [ ] A review-only screenshot matches the mockup.
