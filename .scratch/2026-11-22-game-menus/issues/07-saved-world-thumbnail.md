# 07: Saved World thumbnail

Status: ready-for-agent

Blocked by: [04: Worlds as save slots](./04-worlds-as-save-slots.md), [06: Live
terrain preview](./06-live-terrain-preview.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Creating a World renders its terrain preview once and stores it with the save.
Save slots, Review, Edit World and World details show it; a World saved before
this shows the neutral picture.

## Acceptance criteria

- [ ] A new World's slot shows its thumbnail without regenerating it
- [ ] An older World without a stored thumbnail still lists and opens
- [ ] Deleting a World removes its thumbnail with it

## Comments
