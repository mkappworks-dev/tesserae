# 07: Saved World thumbnail

Status: done

Blocked by: [04: Worlds as save slots](./04-worlds-as-save-slots.md), [06: Live
terrain preview](./06-live-terrain-preview.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Creating a World renders its terrain preview once and stores it with the save.
Save slots, Review, Edit World and World details show it; a World saved before
this shows the neutral picture.

## Acceptance criteria

- [x] A new World's slot shows its thumbnail without regenerating it
- [x] An older World without a stored thumbnail still lists and opens
- [x] Deleting a World removes its thumbnail with it

## Comments

The thumbnail is `thumbnail.png` in the World's folder (`library::THUMBNAIL`), drawn at `Frame::CREATE` (320×180) by a detached task at Create, so the World opens without waiting. A quit within the second or two it takes leaves that World on the neutral picture for good; it is never drawn again. Review and Edit World / World details are left to tickets 10 and 15 (`pages::world_picture`).
