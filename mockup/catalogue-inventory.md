# Catalogue, personal storage and the game menu

This is a UI proposal in the browser prototype. Engine key bindings and
inventory behavior have not been changed.

## Current engine

- Tab toggles Catalogue (`client/actions.rs`, `client/menu.rs`).
- I invokes Help, opening the last controls/settings category (`settings.rs`).
- Escape releases the cursor or closes an overlay; it is not the prototype's new
  full game menu in the engine yet.
- E flips the held Shape. Preserve this existing building control.
- Catalogue is a Material/Shape picker and quick-slot organiser. It does not
  represent ownership or consume a finite carried stack.
- Step 12 provides Carried totals after Pickup. The glossary explicitly states
  that Carried has no slots or limits.

## Proposed prototype navigation

| Input during gameplay | Screen                                  |
| --------------------- | --------------------------------------- |
| Tab                   | Menu (`/menu`)                          |
| Escape                | Menu, or close the current overlay      |
| I                     | Inventory (`/inventory`)                |
| C                     | Catalogue (`/catalogue`)                |
| F10                   | Settings, remembering the last category |
| E                     | Preserve Flip in the engine             |

Inside menus, Tab retains its keyboard-focus role. Text entry owns I/C while
typing. Repeating I or C closes its corresponding item screen; Escape closes an
item screen before opening another menu. Custom dropdowns consume Escape first.

Inventory and Catalogue share an item-oriented frame with labelled tabs, so the
Catalogue is one click away from Inventory. Inventory holds finite example
stacks and the nine-slot hotbar used during play. Catalogue has search,
categories, a Material preview and Shape selection, with no quick slots.

In Creative preview, Add to inventory adds a stack to the backpack, merging with
matching stacks when possible. Open inventory returns to the carried items;
Shift-click or Move to hotbar gives them quick access during play. A full
backpack rejects the addition without changing existing items. Survival preview
keeps the Catalogue read-only.

The example backpack has 27 slots and the hotbar has nine. These are layout
proposals, not engine capacities or approved stack-size rules. Click a stack and
then a destination to move or swap; drag/drop is an alternative. The browser
prototype does not implement crafting, armour or equipment rules.

## Scope

Designing and validating the UI can happen now. The actual personal-storage
system belongs to DESIGN.md step 15, Inventory: Server-owned transactions,
ownership, capacity, stack rules, moves, persistence and history integration.
Crafting belongs to step 16. Catalogue remains useful as a building-content
picker and must not silently turn into an ownership system by changing its name.

Games/Mods supply item and stack rules. The engine owns transactions and sends
authoritative results to the Client. No Client UI may implement real inventory
transfers by mutating World state.

## Reference patterns

Minecraft separates Escape/Menu from E/Inventory. Its survival and creative
experiences distinguish gathered resources from freely available building
content. Borrow that conceptual separation and the predictable grid/quickbar
layout without copying its artwork or its E binding over Tesserae's Flip action.
[Minecraft controls](https://www.minecraft.net/en-us/article/minecraft-controls),
[Creative vs Survival](https://www.minecraft.net/en-us/article/creative-vs-survival-mode).

Luanti defaults inventory to I. Its API distinguishes inventory lists and
locations, including player and world-container inventories. This supports
keeping the player's storage separate from a content catalogue and from future
chests or crafting interfaces.
[Luanti controls](https://docs.luanti.org/for-players/controls/),
[Luanti inventory API](https://api.luanti.org/inventory/).
