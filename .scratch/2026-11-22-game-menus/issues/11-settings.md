# 11: Settings

Status: ready-for-agent

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Settings moves into the frame with its categories as the rail. Choices are ◀
value ▶ selectors and On/Off toggles that left and right change, sliders stay,
and every change applies at once. Display changes open a keep-or-revert prompt
with a ten-second countdown. One help panel explains the highlighted row. Key
bindings show one cap per input with categories on Q and E and mouse buttons as
LMB, RMB and MMB; selecting a cap waits for a key over its row. Esc goes back
and R resets to defaults, both as prompts.

## Acceptance criteria

- [ ] A change applies without an Apply step and survives leaving Settings
- [ ] A display change reverts after ten seconds or on dismissal unless kept
- [ ] Left and right change a focused selector or toggle instead of moving focus
- [ ] Rebinding a cap takes the next key; Esc cancels and Delete clears

## Comments
