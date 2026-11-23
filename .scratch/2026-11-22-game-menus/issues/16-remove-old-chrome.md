# 16: Remove the old header, breadcrumbs and footers

Status: done

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

- [x] No screen renders the old header, breadcrumbs or footer bar
- [x] Nothing references the removed code
- [x] All screen tests pass unchanged

## Comments

- The Mods library and Trust were the last pages on the old header and footer;
  both moved onto the frame per the mockup. Library: "Mods" sheet of cards, no
  buttons, prompts N New World (only with a Game) and Esc Back; the "Installed
  on this device" line, the Mod count, Main menu and Create World buttons are
  gone. Trust: one primary button ("Trust and open" / "Trust all N and
  create"), prompts Enter and Esc Back.
- Decision: Trust now focuses its primary button, so Enter trusts (the mockup's
  "Enter Trust and open"); before, Back had focus. Two tests changed with it,
  and the tab-order test no longer expects Back on Trust or Home/New/Settings
  on the Library.
- Removed: `pages::header`/`footer`/`HeaderButton`, `header_hover`,
  `Act::Home`, `cine::sheet`/`Sheet`/`sheet_header`/`sheet_footer`/`title`,
  `buttons::Align` and `Eyebrow`, and the theme constants only they used.
- Not touched: the in-game structure menu's own card header and footer
  (`structure_menu.rs`) is a panel, not launcher chrome.
