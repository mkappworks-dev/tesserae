# 26: Sky and Fog colour picker

Status: done

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

- [x] Sky and Fog show a swatch and hex, and open the picker by pointer, keys
      and controller
- [x] Presets, square, hue slider and hex field set the colour; the diorama
      redraws live
- [x] Esc restores the starting colour; a click outside keeps the new one
- [x] The picker stays inside the window

## Comments

- The picker lives in `client/worlds/colour.rs`. Its square, hue slider and
  presets are `pad::Surface`s: every arrow, D-pad and stick direction on them
  sends a `pad::Nudge` instead of moving focus, and the popup is a modal
  `TabGroup`, so Tab and LB/RB move between its four parts.
- `BiomesDraft::colours` and the "Write colours as #rrggbb." problem are gone:
  a picker cannot hold an invalid colour. The closed controls repaint from the
  draft every frame, which also fixes the review-fix gap where picking one
  colour on a Biome borrowing the land's left the other showing the
  placeholder until a rebuild.
- Like the mockup, it opens above its row when the panel's visible body has no
  room below, even if the window has; in a short window that can be over the
  Climate section.
