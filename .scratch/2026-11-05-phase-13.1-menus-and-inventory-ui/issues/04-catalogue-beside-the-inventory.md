# 04: Catalogue beside the Inventory

Status: done

Blocked by: 03

Spec: [Phase 13.1](../spec.md).

## What to build

The Catalogue becomes a side panel beside the Inventory: search, categories, and Materials as tiles. Taking a Material gives the Item that places it: drag a tile onto any slot; set a count from 1 to the stack limit and send it to the backpack or the selected hotbar slot; double-click sends to the hotbar slot, Shift-click to the backpack. A stack displaced from a slot moves to the backpack; a full backpack refuses. C opens the Inventory with the Catalogue search focused. Each take is a Server-validated intent. On the first open of a new World, the Client offers the stored per-Game quick slots as takes, and the quick-slot file is no longer written.

## Acceptance criteria

- [x] Each take path (drag, count to backpack, count to hotbar slot, double-click, Shift-click) lands the right Item and count through the Server.
- [x] A take into a full backpack is refused and changes nothing.
- [x] C opens the Inventory with the search focused.
- [x] A new World's hotbar is seeded from the stored quick slots, by name; missing content is skipped.
