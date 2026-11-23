# 23: Light tints as faint as the mockup's

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

The mockup's faint white fills (alpha 0.04–0.08) come out far too light in the
Client because Bevy blends in linear light: the highlighted New World tile
measures (83,84,83) on screen against the mockup's (19,22,22), and Settings'
◀ value ▶ selectors and On/Off toggles are affected too. Every such fill goes
through the same conversion as `theme::css_dim` (an equivalent for light tints)
so it matches the mockup on screen. The title menu, still faintly visible
behind the Settings sheet, is hidden as the mockup hides it.

## Acceptance criteria

- [x] On-screen captures of the Worlds tiles, Settings selectors and toggles
      match the mockup within a few levels
- [x] The title menu does not show through the Settings sheet
- [x] A theme test pins the tint conversion against the mockup's CSS values

## Comments

`theme::css_tint` matches over the sheet as seen on screen (sRGB 0.1), not mid-grey: a tint over near-black lifts far more in linear light. On-screen: unlit World tile 33 = mockup 33, lit New World 40 vs 34, Settings selector 37 vs 44 where the mockup's sheet there is lighter (29 vs 26); over our sheet the CSS blend gives 37.5. Left as they were: the HUD's tints, which sit on the World, and the slot bevel `SLOT_LIFT`.
