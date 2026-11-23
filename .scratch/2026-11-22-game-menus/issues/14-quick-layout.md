# 14: Quick layout

Status: done

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Quick layout is a side panel: a Count or Point-to-point switch, Line, Plane and
Box on 1, 2 and 3, a row per axis with its direction (click to reverse) and −
and + steppers, a gap stepper, and the total out of 256. The same changes are on
keys shown in the prompt bar; Enter builds and Esc exits. The Layout-ready HUD
card shows its keys as text and has nothing to click.

## Acceptance criteria

- [x] Steppers and keys change the same counts
- [x] Going over 256 Voxels is refused with a message
- [x] The HUD card contains no buttons

## Comments

- The panel's controls are pointer-only (`frame::PointerOnly`, no `TabIndex`), so nothing in it takes focus and Enter always builds; keys and the pad go through the prompt bar: 1–3 shape, ←→ / D-pad axis, ↑↓ / D-pad count, R (X) reverse, G gap (Shift+G less, triggers on a pad), P (Y) method, Enter (A) build, Esc (B) exit.
- Esc now leaves the mode, as the mockup's Exit does; it used to close the menu with the mode still on.
- Typing digits into a count is gone: 1–3 are the shapes now.
- While the panel is open the hotbar, minimap and first-person hand are hidden: the prompt bar takes the bottom, the panel the right, and the hand drew over its foot.
- The live "In the World" count and the size/Voxel-size totals box left the panel (not in the mockup); the HUD card still says what the aim would lay, and its keys now read "edit" and "exit".

