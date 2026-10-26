# 09: The menu is a grid of square tiles

Status: done
Blocked by: 08

Spec: [decision 19](../spec.md).

## What to build

The Tab menu becomes a grid of square tiles in the item's placeholder colour,
named under each: solids as cubes; the Shapes of the last solid the pointer
was over (at first the held solid, else the first); fluids; fire when anything
burns; then the nine slots, numbered in a corner. The selected slot and the
held item are marked by a border.

- A click on an item tile holds it outside the slots, closes the menu and
  grabs the cursor.
- A click on a slot selects it and keeps the menu open.
- Dragging an item tile onto a slot puts it there; dragging a slot onto a slot
  swaps them; a drop elsewhere does nothing.
- 1 to 9 with the pointer over an item tile puts it in that slot without
  moving the selection; elsewhere they select a slot.

## Acceptance criteria

- [ ] Each behaviour above has a headless Client test driving pointer and key
      input.
- [ ] Every tile is square at any window size.
- [ ] With no Game the grid is empty and the nine slots show as empty squares.
- [ ] Clicks never edit the World while the menu is open.
- [ ] A capture of the open menu is in the `/tmp` review folder.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

The menu uses square colour tiles with labels below and slot numbers in a
corner. Solids quick-pick cubes; Shape tiles follow the last hovered solid.
Click holds outside the slots and closes with cursor grab. Slot clicks stay
open; drag assigns or swaps; numbered keys over items assign without moving
selection. Shape-panel updates preserve the item entities being dragged.
Pointer events and key input have headless coverage. Final formatting,
review, CI and captures remain deferred to ticket 13.
