# 21: Window mode in Display settings

Status: done

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

Settings › Display gains a Window mode selector: Windowed, Borderless
fullscreen and Fullscreen, as in the approved mockup, using Bevy's
`WindowMode`. It applies at once and is a display change, so it opens the
keep-or-revert prompt like the others. The choice is saved with the other
settings and restored at startup. Borderless fullscreen is the bar-free option
on Windows and Linux.

## Acceptance criteria

- [x] The selector changes the window mode at once and the keep-or-revert
      prompt reverts it on timeout
- [x] The mode is saved and restored at startup
- [x] Left and Right change it like the other selectors

## Comments

The macOS title bar fields need no re-assert on the way back to Windowed:
winit enters both fullscreens through AppKit's own toggle, which keeps the
window's style mask. Window mode, like the other display settings, is not
applied under `TESSERAE_SCREENSHOT`.
