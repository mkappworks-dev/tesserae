# 12: Inventory, Catalogue, Crafting and Chest

Status: ready-for-agent

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

The Inventory is one panel without a title bar or close button; I and Esc close
it. A tooltip on the hovered or focused slot shows the name, count and maximum,
and which Shapes the item places as, read-only. Click picks up and places,
Shift-click quick-moves, right-click splits, Q drops one and Shift-Q the stack,
and 1–9 send to the hotbar, all as prompts. The Catalogue and recipes are icon
grids with names in tooltips; click takes a stack or crafts one, Shift-click
takes one or crafts all. A Chest sits above the backpack in the same panel, with
Take all and Sort as prompts.

## Acceptance criteria

- [ ] Each input action moves stacks as its prompt says
- [ ] The tooltip names the item and lists its Shapes without offering to change
      them
- [ ] Uncraftable recipes are greyed and list what is missing
- [ ] Shift-click moves a stack between a Chest and the backpack

## Comments
