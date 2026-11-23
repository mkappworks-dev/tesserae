# 12: Inventory, Catalogue, Crafting and Chest

Status: done

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

- [x] Each input action moves stacks as its prompt says
- [x] The tooltip names the item and lists its Shapes without offering to change
      them
- [x] Uncraftable recipes are greyed and list what is missing
- [x] Shift-click moves a stack between a Chest and the backpack

## Comments

- Mouse caps (Click, ⇧ Click, Right-click) and 1–9 are named-only prompts, a
  new `frame::Cap::Text`; Q, S and T fire their prompts, and `/` opens the
  hidden search. The Close prompt shows the Inventory binding's key and is not
  clickable, so it never closes twice with the toggle; Esc Back is clickable.
- The slot prompts follow the mockup: plain Click (pick up and place) and Shift-Q
  have no prompt of their own, though both work.
- Drop is Y and Sort is X on a controller; the tooltip and the keys act on the
  hovered control, else the focused one, so a controller acts on focus.
- The prompt bar's place is blank: the in-game screens have no Game and World
  name to hand yet.
- Shift-click Chest ⇄ backpack is tested as the `QuickTransfer` intent it sends;
  the Server's Chest tests already prove the move.
- `frame::plugin` is now added by the Client plugin, so every in-game screen has
  it.
