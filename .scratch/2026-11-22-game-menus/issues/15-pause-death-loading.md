# 15: Pause, World details, death and loading

Status: done

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md), [07: Saved World
thumbnail](./07-saved-world-thumbnail.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

The pause menu is a plain list under 'Paused' with key caps for Resume,
Inventory and Map, and the you-are-here corner. World details shows the World's
thumbnail, its fixed settings in two columns, a seed copy control and the
enabled biomes. The death screen shows the cause, where the Player fell and how
long dropped items stay, with Respawn showing its Enter cap. The loading screen
is full screen with the Game and World name, a tip and a slim bar.

## Acceptance criteria

- [x] The pause menu opens with Resume focused
- [x] World details opens with Close focused and closes on Esc
- [x] Esc does not leave the death screen
- [x] The loading screen fades out when the ground around the Player is drawn

## Comments

- World details shows no Mods or Created rows: `WorldFacts` carries neither,
  and the biome chips have no swatch colour for the same reason. Adding them
  means widening the Server's `WorldFacts` message.
- Loading tips name no keys, unlike the mockup's, so a rebound key or a
  controller never makes one wrong.
- The death screen drops Save & Quit, as the mockup does: Respawn and Return to
  Worlds only.
