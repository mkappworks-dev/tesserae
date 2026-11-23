# 08: Mods step

Status: done

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

- [x] Filter and sort change the list; the select-all checkbox acts only on the
      Mods shown
- [x] Required Mods cannot be cleared
- [x] Changing the selection after trusting asks for trust again on continue
- [x] Cancelling the trust modal stays on the Mods step

## Comments

The trust modal is the Game step's `Screen::StepTrust` dialog; the inline panel
is gone. "Asks again" holds whenever the new selection contains an untrusted
item: Trust is recorded per build, so dropping a Mod asks nothing.

A popup closes on Esc, on its icon, on picking a sort, or on any other control;
a click on empty space leaves it open. Filters persist across steps (the mockup
resets them on a step change). Mods with many Materials show at most 8 swatches,
the width of a thumbnail.
