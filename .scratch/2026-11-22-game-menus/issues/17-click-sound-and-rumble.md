# 17: Click sound and controller rumble

Status: ready-for-agent

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Split from 02 to keep that ticket to one context. Every menu press that 02's
press feedback covers also plays a single click sound, and a press made on a
controller rumbles it briefly. The sound and rumble hang off the same press
event as the shrink-and-darken, so no screen wires them by hand.

## Acceptance criteria

- [ ] A click sound plays on every press, by key, click or controller
- [ ] A controller press also rumbles the controller briefly; a key or mouse
      press does not
- [ ] Neither delays the action

## Comments
