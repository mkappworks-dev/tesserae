# 16: Remove the old header, breadcrumbs and footers

Status: ready-for-agent

Blocked by: [04: Worlds as save slots](./04-worlds-as-save-slots.md), [10:
Review and Edit World](./10-review-and-edit-world.md), [11:
Settings](./11-settings.md), [12: Inventory, Catalogue, Crafting and
Chest](./12-inventory.md), [13: Map](./13-map.md), [14: Quick
layout](./14-quick-layout.md), [15: Pause, World details, death and
loading](./15-pause-death-loading.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Once every screen uses the new frame and prompt bar, delete the old header bar,
breadcrumbs, footer bars and their styles and code paths.

## Acceptance criteria

- [ ] No screen renders the old header, breadcrumbs or footer bar
- [ ] Nothing references the removed code
- [ ] All screen tests pass unchanged

## Comments
