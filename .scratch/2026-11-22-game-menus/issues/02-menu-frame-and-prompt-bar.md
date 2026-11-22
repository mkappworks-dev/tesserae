# 02: Menu frame and prompt bar, on the title screen

Status: ready-for-agent

Blocked by: [01: Approve the mockup](./01-mockup-approval.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

The Client gains the shared menu frame (title over the scenery, no header bar or
breadcrumbs, at most one main button) and the prompt bar (clickable prompts that
answer their key), focus-first screens with hover moving focus, and the standard
press feedback with a click sound and controller rumble. The title screen is the
first user: a logo beside Tesserae and one menu of Continue (with the last
World's name), Worlds, Create World, Mods, Settings and Quit, with the version
in a corner.

## Acceptance criteria

- [ ] The title screen opens with Continue focused and names the last-played
      World
- [ ] Hovering an item moves focus to it; arrows and the D-pad still move it
- [ ] A letter prompt fires on its key and on click; it does not fire while a
      text field has focus
- [ ] A press shrinks and darkens the button immediately and the action runs
      without delay
- [ ] A click sound plays on press; a controller press also rumbles briefly
- [ ] On a controller, A and B act as Enter and Esc and X and Y fire the
      prompts that name them; the bar shows controller buttons after controller
      input and keys again after a key press or mouse move
- [ ] The old header and footer remain for screens not yet moved

## Comments
