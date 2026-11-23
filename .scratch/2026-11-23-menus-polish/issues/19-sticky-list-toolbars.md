# 19: Sticky list toolbars

Status: done

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

Where a screen lists items under a toolbar, the toolbar stays in place while
the list scrolls under it, as in the approved mockup: the search field with its
filter and sort buttons, and the select-all row with Reset all. This covers the
Mods step, the Biomes step, the Worlds search once it appears, and any other
list the mockup gives the same toolbar.

## Acceptance criteria

- [x] Scrolling the Mods or Biomes list leaves search, filter, sort, select-all
      and Reset all in view
- [x] The same holds on every other screen with a list toolbar
- [x] Focus moving down the list scrolls it without hiding the focused row
      under the toolbar

## Comments

- Mockup pages with a sticky toolbar: `/worlds/new/mods`, `/worlds/new/biomes`
  (the plain list and the list beside the open Biome panel), `/worlds` once its
  search appears, and the `/map` legend tabs. The select-all row moved out of
  the re-rendered results into the toolbar node, so one node is sticky. `/mods`
  and `/inventory` need nothing.
- Done: the list under each toolbar is now the page's `PageScroll` node, so it
  alone scrolls and keeps its place across a rebuild; the sheet body above it
  holds the toolbar still. The Biome panel's cap is now the split's height
  instead of `62vh`. The `/map` legend tabs were left to the map lane.
