# 09: Darker dim behind overlays

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

Every menu overlay and dialog dims the scenery as much as the mockup does, including behind World details.

## Acceptance criteria

- [x] Settings, Create World, dialogs and World details dim to the mockup's level, checked by capture

## Comments

The dim was the mockup's sRGB alphas applied in Bevy UI, which blends in linear
light, so each dimmed too little. `theme::css_dim` converts a CSS alpha against
a mid-grey backdrop (the rule `SHEET_SHADE` already followed): the dialog
overlay goes 0.70 → 0.89, the shade 0.48/0.24 → 0.65/0.37, the pause shade
0.75 → 0.92. The stronger-contrast overlay (0xe6) is now only a little darker
than the normal one (0xe2); left as is. Offscreen captures still show the
Settings sheet lighter than the mockup: that is the sheet's own fill, already
converted, and the known offscreen translucency.
