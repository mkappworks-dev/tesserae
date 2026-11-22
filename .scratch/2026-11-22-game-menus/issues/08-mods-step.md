# 08: Mods step

Status: ready-for-agent

Blocked by: [03: Optional thumbnail in mod.toml](./03-mod-thumbnail-field.md),
[05: Create World: Game step and titles](./05-create-world-game-step.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

The Mods step lists Mods with their thumbnails and summaries. Filter (by
category and selected-only) and Sort (name, category, selected first) open as
popups from icons beside the search field, a filter count badges the icon, and
Reset is enabled only when a filter is set. One tri-state checkbox selects or
clears every Mod shown; required Mods stay on. Continuing with untrusted Mods
opens a trust modal listing each item, its build and whether it runs code.

## Acceptance criteria

- [ ] Filter and sort change the list; the select-all checkbox acts only on the
      Mods shown
- [ ] Required Mods cannot be cleared
- [ ] Changing the selection after trusting asks for trust again on continue
- [ ] Cancelling the trust modal stays on the Mods step

## Comments
