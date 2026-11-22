# 15: Pause, World details, death and loading

Status: ready-for-agent

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

- [ ] The pause menu opens with Resume focused
- [ ] World details opens with Close focused and closes on Esc
- [ ] Esc does not leave the death screen
- [ ] The loading screen fades out when the ground around the Player is drawn

## Comments
