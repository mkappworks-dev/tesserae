# 12: Sheets hug their content; Mods step scrolls

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

A framed sheet is as tall as its content, up to the screen, and scrolls inside past that, as in the mockup; Settings is rechecked. The Mods step list scrolls with the wheel, keys and D-pad.

## Acceptance criteria

- [x] Worlds with two tiles and the Trust page hug their content
- [x] Settings and Create World still keep Next and the main button in one place
- [x] The Mods step list scrolls to its last Mod by wheel and by focus

## Comments

No screen scrolled by the wheel before this: `frame` now scrolls the innermost framed list under the pointer, 48 px a notch. Focus moved by a key scrolls into view as the controller's did; focus the pointer moved does not, so hovering a half-shown card does not jump the list. Edit World hugs its content too, as in the mockup, since it is not stepped.
