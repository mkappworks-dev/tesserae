# 04: Worlds as save slots

Status: done

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Worlds is a grid of save-slot tiles in the new frame, New World first. Enter
plays, E edits and Delete deletes the highlighted World, and Esc goes back, all
as prompts. A search field appears only once the Worlds no longer fit on one
screen. Until the saved World thumbnail exists, a tile shows a neutral picture.

## Acceptance criteria

- [x] New World is the first tile and is focused on entry
- [x] E and Delete act on the highlighted World; Delete still confirms
- [x] Hovering or focusing a World shows Edit and Delete icons on its tile;
      the D-pad moves between tiles, not onto the icons
- [x] On a controller, X edits and Y deletes the highlighted World
- [x] No search field with eight or fewer Worlds; one appears beyond that
- [x] No breadcrumb, card menus or footer status text remain on this screen

## Comments

- Each World slot shows the old neutral landscape (`pages::thumbnail`) in a node marked `pages::SlotPicture`; ticket 07 swaps in the saved terrain thumbnail there.
- A World whose Mods are unmet keeps a focusable slot (so E and Delete reach it) that shows why and does nothing on Play, instead of hiding Play.
- The Worlds sheet body no longer keeps its scroll position across a rebuild: `frame::menu_sheet` has no `PageScroll`. Rebuilds on this screen are rare now that the card menus are gone.
- The card popover (`cinematic::menu_item`, `Act::Menu`, `MENU_*` colours) had no other user and is deleted.
