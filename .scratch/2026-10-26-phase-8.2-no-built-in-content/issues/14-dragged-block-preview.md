# 14: A block preview follows a drag into a quick slot

Status: done
Blocked by: 13

User follow-up on 2026-10-26 for PR #21.

## What to build

An item or slot drag carries its model under the pointer, preserving the
Shape and orientation of a slot. The follower does not intercept picking.
The destination slot signals that it accepts the drop and shows the new
item immediately on drop. Release elsewhere and menu close remove the
follower. The menu stays open and dragging never quick-picks the item.

## Acceptance criteria

- [x] Pointer drag events move the model in UI coordinates, including UI scale.
- [x] Item assignment and slot swapping update the destination model.
- [x] Drop/cancel/menu close remove the follower without editing the World.
- [x] Formatting, relevant tests and final full CI pass.

## Comments

The user requested this after accepting the hover and fixed-footer update.
The follower reuses the shared content-model atlas. Final combined visual
review follows the material texture work in ticket 15.

### 2026-10-26: implementation

The drag captures the source Holding, draws a non-pickable model from the
shared atlas under the scaled pointer, and highlights accepting slots in
green. Drop clears the follower immediately and rebuilds the destination
model; menu close cancels a drag. Tests drive both item and turned-slot
drags through movement and drop, including a non-default UI scale, and
check cancellation. All 95 Client tests pass. Formatting is complete;
combined visual review and full CI follow ticket 15, per the user's
implement-first workflow.

### 2026-10-26: combined validation

The combined texture and drag tree passes 885 tests with five existing
manual measurements ignored. The dev capture path now drives the real
Pointer drag/drop observers to show a water model over slot 5, its green
drop highlight, and the slot changing from lamp to water. Capture review
is tracked with ticket 15; files stay in the review folder until accepted.
