# 08: Biomes panel and dioramas

Status: ready-for-agent

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

The Biomes side panel matches the approved mockup (compact, with the sliders and Caves). Each biome's picture is a small isometric diorama drawn from its own settings and redrawn when they change. The rail joins the sheet as on the other steps. The climate map is 2:1, and Surface, Soil and Rock are dropdowns showing each Material's colour swatch, with a search field at the top of the list (A opens, D-pad moves, A picks).

## Acceptance criteria

- [ ] The panel matches the approved mockup
- [ ] A biome's picture is a diorama that redraws after an edit
- [ ] The Biomes rail is part of the sheet
- [ ] Surface, Soil and Rock are dropdowns with swatches and search, usable by controller

## Comments

A Ground dropdown's open list keeps its full height while search filters it (user decision), so a list opening upward stays attached to its field.
