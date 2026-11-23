# 04: Edit World asks before discarding

Status: done

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

Esc Cancel on Edit World with unsaved edits opens a Discard changes? dialog matching the mockup, Keep editing focused. Without edits, Esc leaves at once.

## Acceptance criteria

- [x] Esc with unsaved edits opens the dialog with Keep editing focused
- [x] Discard returns to Worlds without saving; Keep editing returns to the form
- [x] Esc without edits returns to Worlds

## Comments

- `Screen::DiscardEdits(path)` reuses `Act::KeepEditing` from Create World's Leave this setup? dialog and adds `Act::DiscardEdits`. Esc on the dialog keeps editing, like Save changes first?.
