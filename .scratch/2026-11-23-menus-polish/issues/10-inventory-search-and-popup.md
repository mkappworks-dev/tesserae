# 10: Inventory search field and popups in full

Status: done

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

The Inventory's search is a field under the Catalogue and Crafting tabs, as in the approved mockup. An item's popup in the Catalogue, Crafting and workstation panes shows in full instead of being clipped at the pane's edge.

## Acceptance criteria

- [x] The search field sits under the tabs and filters as it types
- [x] / focuses it; it does not take letter prompts while focused
- [x] A popup near a pane edge is shown whole

## Comments

A tooltip takes `OverrideClip`, so the scrolling grid no longer cuts it; it is
not moved to stay inside the window. The field keeps the shared `/` hint from
`menu_ui::search_frame`, which the mockup does not show. A Chest's prompt set
has no `/ Search`, as in the mockup.
