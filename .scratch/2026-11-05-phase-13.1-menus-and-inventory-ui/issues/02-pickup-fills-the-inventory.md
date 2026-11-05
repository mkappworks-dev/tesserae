# 02: Pickup fills a Server-owned Inventory

Status: ready-for-agent

Blocked by: 01

Spec: [Phase 13.1](../spec.md).

## What to build

Pickup puts Items into slots. A pure Inventory module in `tesserae-core` holds 27 backpack and 9 hotbar slots and owns the fit rules: merge into matching stacks, then empty slots, hotbar before backpack, returning what did not fit. The Player's Inventory replaces Carried on the Server; what does not fit stays on the ground as a Dropped item with its count reduced. The Inventory is saved as a Player extension. A save with Carried totals packs them into slots once and drops any overflow beside the Player. The Server sends the full Inventory whenever it changes, and what a Pickup left behind. The Client replaces the "carried:" line and the "+N" toast with the pickup feed (bottom right, newest on top, the same Item merges and moves to the top, rows fade after 3 s) and the "Inventory full · N Item left on the ground" notice above the hotbar, cleared when a slot frees. Stack limit is 64 until ticket 08.

## Acceptance criteria

- [ ] Pickup merges into matching stacks first, then empty slots, hotbar before backpack.
- [ ] A Pickup into a full Inventory takes what fits; the rest stays as a Dropped item with the reduced count, and the notice names it.
- [ ] The Inventory survives save and reopen; a phase 13 save with Carried opens with those Items in slots and the overflow dropped beside the Player, and Carried is not written again.
- [ ] The pickup feed and the notice follow Server messages only; no Client system changes the Inventory.
- [ ] Unit tests cover the Inventory module's fit and merge rules; Server tests cover Pickup through the headless App.
