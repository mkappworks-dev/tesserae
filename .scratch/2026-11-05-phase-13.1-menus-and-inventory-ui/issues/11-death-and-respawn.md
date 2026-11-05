# 11: Death and respawn

Status: done

Blocked by: 07, 09

Spec: [Phase 13.1](../spec.md).

## What to build

At zero health the Player dies. The Server records the cause, the position in metres and the dropped stacks; drops the whole Inventory there as Dropped items unless the Game declares keep_inventory; clears the Player's undo history; and ignores movement and edit intents until respawn. The death is saved, so reopening the World shows it. The Client shows the centred death screen over the darkened World: the World and distance from spawn, the cause, what it means, where the Items lie and for how long, then Respawn (Enter) and Return to Worlds; Escape does nothing there. Respawn uses the existing respawn intent at the World spawn with full Vitals, and the top-right notice says how far away the Items are, for 5 minutes.

## Acceptance criteria

- [x] Death drops every stack as Dropped items at the death point; with keep_inventory nothing drops.
- [x] Intents sent while dead are ignored.
- [x] Quitting while dead and reopening shows the death screen.
- [x] Respawn restores full Vitals at the World spawn and shows the distance notice.
- [x] Undo after a respawn has nothing from before the death.

## Comments

- Maintainer, 2026-11-05: the death screen shows a live despawn countdown. The Death records the ids of the Dropped items it made, saved in `death_v1` as `drops`, and the Server reports the whole seconds left on the first of them still lying there, from their own despawn timers (which pause while their Chunk is not held), re-sending the report each time that passes a second. The screen says "They stay for 4:12." and rewrites only that line as it ticks, and the respawn notice gives the same time. A reopened World continues from the saved timers, not from 5 minutes. Kept or empty Inventories say nothing about despawn; drops that despawn while the Player is still dead are said to have despawned.
