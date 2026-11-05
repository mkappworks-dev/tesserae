# 11: Death and respawn

Status: ready-for-agent

Blocked by: 07, 09

Spec: [Phase 13.1](../spec.md).

## What to build

At zero health the Player dies. The Server records the cause, the position in metres and the dropped stacks; drops the whole Inventory there as Dropped items unless the Game declares keep_inventory; clears the Player's undo history; and ignores movement and edit intents until respawn. The death is saved, so reopening the World shows it. The Client shows the centred death screen over the darkened World: the World and distance from spawn, the cause, what it means, where the Items lie and for how long, then Respawn (Enter) and Return to Worlds; Escape does nothing there. Respawn uses the existing respawn intent at the World spawn with full Vitals, and the top-right notice says how far away the Items are, for 5 minutes.

## Acceptance criteria

- [ ] Death drops every stack as Dropped items at the death point; with keep_inventory nothing drops.
- [ ] Intents sent while dead are ignored.
- [ ] Quitting while dead and reopening shows the death screen.
- [ ] Respawn restores full Vitals at the World spawn and shows the distance notice.
- [ ] Undo after a respawn has nothing from before the death.
