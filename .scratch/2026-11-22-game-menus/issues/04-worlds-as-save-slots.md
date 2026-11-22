# 04: Worlds as save slots

Status: ready-for-agent

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Worlds is a grid of save-slot tiles in the new frame, New World first. Enter
plays, E edits and Delete deletes the highlighted World, and Esc goes back, all
as prompts. A search field appears only once the Worlds no longer fit on one
screen. Until the saved World thumbnail exists, a tile shows a neutral picture.

## Acceptance criteria

- [ ] New World is the first tile and is focused on entry
- [ ] E and Delete act on the highlighted World; Delete still confirms
- [ ] Hovering or focusing a World shows Edit and Delete icons on its tile;
      the D-pad moves between tiles, not onto the icons
- [ ] On a controller, X edits and Y deletes the highlighted World
- [ ] No search field with eight or fewer Worlds; one appears beyond that
- [ ] No breadcrumb, card menus or footer status text remain on this screen

## Comments
