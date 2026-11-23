# 26: Sky and Fog colour picker

Status: ready-for-agent

Blocked by: None (can start immediately); start after the review-fix lane
merges, since both change the Biomes step's Sky and Fog rows

Spec: [Menus polish](../spec.md).

## What to build

The Biomes edit panel's Sky and Fog rows become the approved mockup's colour
picker (`/worlds/new/biomes`, a biome's pencil, then Sky or Fog;
`mockup/controls.js`): closed, a swatch with its hex value as one control;
opened by click, Enter, Space or A, a popup (`frame::Popup`) with eight presets,
a saturation/brightness square, a hue slider and a hex field. Arrows move within
the focused part, Tab (LB/RB) moves between parts, Enter picks, Esc restores the
colour it opened with, a click outside keeps the new colour, and a preset picks
and closes. The biome's diorama and the terrain preview redraw live. It opens
above its row when it would run off the panel. Today the rows are hex text
fields.

## Acceptance criteria

- [ ] Sky and Fog show a swatch and hex, and open the picker by pointer, keys
      and controller
- [ ] Presets, square, hue slider and hex field set the colour; the diorama
      redraws live
- [ ] Esc restores the starting colour; a click outside keeps the new one
- [ ] The picker stays inside the window

## Comments
