# 14: Quick layout

Status: ready-for-agent

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

- [ ] Steppers and keys change the same counts
- [ ] Going over 256 Voxels is refused with a message
- [ ] The HUD card contains no buttons

## Comments
