# 24: Q quits from the title screen and pause menu

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

As in the mockup, the title screen's prompt bar gains a clickable `Q Quit`
prompt and the pause menu's gains `Q Save & Quit`. Q fires them as a letter
prompt (not while a text field has focus), going through the same path as the
menu items, so the unsaved-quit confirmation still applies.

## Acceptance criteria

- [ ] Q on the title screen quits, as the Quit item does
- [ ] Q on the pause menu saves and quits, as Save & Quit does
- [ ] Both prompts show in the prompt bar and work on click

## Comments
