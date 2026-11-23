# 13: Popups close on a click elsewhere

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

A click outside an open popup closes it, as in the mockup: the Mods and Biomes
filter and sort popups, and every other popup a menu opens (Map popovers,
Settings selectors, Inventory menus). The click that closes a popup does
nothing else, so it cannot press a control under it by accident. One shared
rule in the frame, not one per screen.

## Acceptance criteria

- [x] A click outside the Mods or Biomes filter or sort popup closes it
- [x] Every other menu popup closes the same way
- [x] The closing click does not activate what lies under it
- [x] Esc and the popup's own button still close it

## Comments

The Client's only popups are the Mods and Biomes filter and sort popups and the
Map's add and edit popover. Settings' selectors are inline ◀ value ▶ steppers
and its disclosures open in place; the Inventory has hover tooltips, not menus;
so none of them has a popup to close.

A popup marks itself `frame::Popup` and answers `frame::Dismiss`. The frame
lays a transparent scrim over the whole screen just under the popup, so the
press that closes it lands on the scrim and nothing under it sees it, the
popup's own icon included. On the Map this differs from the mockup, where a
click on the Map with the add popover open moves its point and a click on
another mark opens that one: here the first click closes the popover and the
next places or opens. Wheel and drag on the Map wait for it to close too.
