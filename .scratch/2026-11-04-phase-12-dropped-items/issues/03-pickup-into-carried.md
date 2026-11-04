# 03: Walking near a drop picks it up

Status: done

Blocked by: 02

## What to build

A Dropped item past its 0.5 s delay and within 1.5 m of the Player's body is
removed and added to that Player's Carried count by name. Carried is Server
state, published to the Client each tick as plain data. It is not an inventory, and the engine constants (0.5 s delay, 1.5 m radius)
are not declarable.

## Acceptance criteria

- [x] Pickup inside the radius moves the whole stack into Carried and removes
      the Entity; outside the radius or before the delay it does nothing.
- [x] Counts saturate rather than wrap.
- [x] The Server writes spawned, moved, removed and Carried messages every
      tick; a frame with no tick drops none.
- [x] No Client system takes mutable access to entity state.
- [x] Pickup enters no undo history and is not a Use.
- [x] Tests at the headless Server seam.
