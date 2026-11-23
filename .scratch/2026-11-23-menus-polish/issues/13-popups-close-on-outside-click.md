# 13: Popups close on a click elsewhere

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

A click outside an open popup closes it, as in the mockup: the Mods and Biomes
filter and sort popups, and every other popup a menu opens (Map popovers,
Settings selectors, Inventory menus). The click that closes a popup does
nothing else, so it cannot press a control under it by accident. One shared
rule in the frame, not one per screen.

## Acceptance criteria

- [ ] A click outside the Mods or Biomes filter or sort popup closes it
- [ ] Every other menu popup closes the same way
- [ ] The closing click does not activate what lies under it
- [ ] Esc and the popup's own button still close it

## Comments
