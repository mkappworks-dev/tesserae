# 07: Undo pays and refunds Items

Status: done

Blocked by: 06

Spec: [Phase 13.1](../spec.md).

## What to build

Undo composes with the Inventory. An undo entry records the Item it paid or the Dropped item it produced, and whether it was free. Undoing a paid placement refunds the Item; with no room the refund drops at the Player's feet. Undoing a removal despawns its Dropped item if it still exists, otherwise spends the Item, and is refused if the Player holds none. Redo mirrors both. Restore checks the Inventory with the Voxel preconditions and applies all of it or none of it. Entries made in Creative stay free after a switch to Survival.

## Acceptance criteria

- [x] Undo then redo of a Survival placement returns the Inventory to each prior state.
- [x] A refund with no room appears as a Dropped item at the Player.
- [x] A removal undo with the drop still present despawns it and spends nothing; without the drop and without the Item it is refused and nothing changes.
- [x] A Layout undo refunds every Voxel's Item as one group.
- [x] Creative entries never touch the Inventory.
