# 19: Sticky list toolbars

Status: ready-for-agent

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

Where a screen lists items under a toolbar, the toolbar stays in place while
the list scrolls under it, as in the approved mockup: the search field with its
filter and sort buttons, and the select-all row with Reset all. This covers the
Mods step, the Biomes step, the Worlds search once it appears, and any other
list the mockup gives the same toolbar.

## Acceptance criteria

- [ ] Scrolling the Mods or Biomes list leaves search, filter, sort, select-all
      and Reset all in view
- [ ] The same holds on every other screen with a list toolbar
- [ ] Focus moving down the list scrolls it without hiding the focused row
      under the toolbar

## Comments
