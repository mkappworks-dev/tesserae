# 24: Q quits from the title screen and pause menu

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

As in the mockup, the title screen's prompt bar gains a clickable `Q Quit`
prompt and the pause menu's gains `Q Save & Quit`. Q fires them as a letter
prompt (not while a text field has focus), going through the same path as the
menu items, so the unsaved-quit confirmation still applies.

## Acceptance criteria

- [x] Q on the title screen quits, as the Quit item does
- [x] Q on the pause menu saves and quits, as Save & Quit does
- [x] Both prompts show in the prompt bar and work on click

## Comments

The pause menu's modal `TabGroup` moved from the item list to the root: the frame refuses prompt keys from inside a modal group the prompt is not in, so Q could not fire while the list held focus. The Q prompt carries `Choice::SaveQuit`, which `save::activate` handles as the Save & Quit item's `SaveAction::SaveQuit`.
