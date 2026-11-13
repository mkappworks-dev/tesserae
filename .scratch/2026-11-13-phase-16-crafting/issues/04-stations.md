# 04: Stations

Status: done

Blocked by: 02, 03

Spec: [Phase 16](../spec.md).

## What to build

A player places the Workbench they crafted, aims at it ("G Use Workbench"),
presses G, and crafts a Chest and a Door from Wood. `voxel-uses` gains the
`station` kind with a tag (`hand` reserved). Use on a Station opens a Station
session; the Client opens the Station pane titled by the Station and listing
only that tag's Recipes, as the mockup shows. A Craft of a Station Recipe is
allowed only while the Player has a session at a Station of that tag. The
session closes when the Player leaves Edit reach or the Voxel changes, and the
Client closes the pane with a notice. Terra's Workbench becomes a Station with
its Recipes (Chest, Door and others, 10 to 15 Recipes in all with the hand
ones).

## Acceptance criteria

- [x] A Station Recipe is refused without a session and allowed with one at a
      Station of its tag; two Materials sharing a tag both work.
- [x] The Station pane lists only its tag's Recipes and follows the mockup.
- [x] Walking out of reach or breaking the Station closes the pane with a
      notice.
- [x] A Station declaring the tag `hand` is a load error.
- [x] Terra: Wood, then a Workbench, then a Chest and a Door works in a new
      Survival World.
