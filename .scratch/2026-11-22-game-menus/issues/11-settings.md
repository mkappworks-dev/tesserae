# 11: Settings

Status: done

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

- [x] A change applies without an Apply step and survives leaving Settings
- [x] A display change reverts after ten seconds or on dismissal unless kept
- [x] Left and right change a focused selector or toggle instead of moving focus
- [x] Rebinding a cap takes the next key; Esc cancels and Delete clears

## Comments

The Key bindings search is gone, as in the mockup; categories on Q and E replace
it. Delete can no longer be bound, since it clears a waiting cap. Controller
users switch binding categories by focusing the Movement, Building and Interface
tabs; Q and E have no controller button. ↺ on a row restores both its keyboard
and controller defaults; only the keyboard's can raise a conflict. Reset to
defaults and ↺ do not open the keep-or-revert prompt. Left and Right step a
focused selector through a new `pad::Stepper` marker and `pad::Step` event,
which other screens can reuse. Key caps show the layout's names, so `=` reads
"Equal" as it did before.
