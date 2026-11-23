# 08: Biomes panel and dioramas

Status: done

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

The Biomes side panel matches the approved mockup (compact, with the sliders and Caves). Each biome's picture is a small isometric diorama drawn from its own settings and redrawn when they change. The rail joins the sheet as on the other steps. The climate map is 2:1, and Surface, Soil and Rock are dropdowns showing each Material's colour swatch, with a search field at the top of the list (A opens, D-pad moves, A picks).

## Acceptance criteria

- [x] The panel matches the approved mockup
- [x] A biome's picture is a diorama that redraws after an edit
- [x] The Biomes rail is part of the sheet
- [x] Surface, Soil and Rock are dropdowns with swatches and search, usable by controller

## Comments

A Ground dropdown's open list shrinks to its search results (user decision); a list that opens above its field hangs from its bottom edge so it stays attached.

Done. The diorama is `tesserae_core::diorama::paint`, Bevy-free, painted on the async compute pool and cached by look; it draws the Biome's own ground, sky, fog and caves (cave mouths in the rock face), with no per-Biome scenery such as trees or water, which the mockup keys by Biome name. The rail already joined the sheet on the integration branch; nothing changed there. Sky and Fog stay hex fields on their own lines: the Client has no colour picker. Terra's selection declares two Materials named Stone, Grass and Leaves (`strata:` and `terra:`), so a shared name shows its Mod (`Stone · Terra`), and a list opens scrolled to the chosen Material. The Ground lists also include plants and crops (saplings, wheat stages): the server's preview lists every solid Material, which is outside this ticket. Opening a dropdown rebuilds the page, so the panel's scroll position is now kept across rebuilds.
