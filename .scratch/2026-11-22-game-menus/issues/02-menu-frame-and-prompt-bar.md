# 02: Menu frame and prompt bar, on the title screen

Status: done

Blocked by: [01: Approve the mockup](./01-mockup-approval.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

The Client gains the shared menu frame (title over the scenery, no header bar or
breadcrumbs, at most one main button) and the prompt bar (clickable prompts that
answer their key), focus-first screens with hover moving focus, and the standard
press feedback. The click sound and rumble are
[17](./17-click-sound-and-rumble.md). The title screen is the
first user: a logo beside Tesserae and one menu of Continue (with the last
World's name), Worlds, Create World, Mods, Settings and Quit, with the version
in a corner.

## Acceptance criteria

- [x] The title screen opens with Continue focused and names the last-played
      World
- [x] Hovering an item moves focus to it; arrows and the D-pad still move it
- [x] A letter prompt fires on its key and on click; it does not fire while a
      text field has focus
- [x] A press shrinks and darkens the button immediately and the action runs
      without delay
- [x] On a controller, A and B act as Enter and Esc and X and Y fire the
      prompts that name them; the bar shows controller buttons after controller
      input and keys again after a key press or mouse move
- [x] The old header and footer remain for screens not yet moved

## Comments

The frame is `client/frame.rs`: mark a screen root `Frame` and use
`prompt_bar`, `list_item`, `menu_sheet` and `Autofocus`; `Press` is the event
17 hooks. The title keeps the mockup's three prompts (Tab, Enter, Esc) as plain
hints, since it has no letter actions. Continue is disabled, not hidden, while
its World's Game or Mods are missing, and the missing ones are listed under the
menu. The title no longer shows the last World's Game and play time; the
mockup shows only its name. Pad navigation skips prompts, so the D-pad never
lands in the bar.
