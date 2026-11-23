# 18: Sheet panels as dark as the mockup's

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

On a real screen the Settings sheet is clearly lighter than the mockup's: the
scenery shows through it (sampled about (69,70,71) against the mockup's
(19,24,25) in the rows, (34,35,36) against (16,18,19) in the rail). Ticket 09
fixed the dim around overlays but not the sheet panels themselves. Every framed
sheet (Settings, Create World, Worlds, Edit World, Mods library, Trust) and its
rail reaches the mockup's darkness on screen, using `theme::css_dim` for any
alpha copied from the mockup's CSS.

## Acceptance criteria

- [x] On-screen captures of Settings and Create World match the mockup's panel
      and rail darkness within a few levels
- [x] Panels stay readable over bright and dark scenery
- [x] A theme test pins the panel and rail colours to the mockup's CSS values

## Comments

On screen, before → after → mockup (median sRGB): Settings sheet (47,54,40) →
(24,31,28) vs (27,34,32), rail (16,19,18) → (12,13,13) vs (17,19,18); Create
World sheet (47,55,41) → (27,32,24) vs (28,32,27). The sheet and both rails now
use `css_dim` at the mockup's 0.9 → 0.78; the focused list item keeps the old
fill as `theme::LIT_ITEM`. The mockup's `backdrop-filter: blur(6px)` has no
Bevy equivalent and is not drawn.

Left for a follow-up: white tints copied from the CSS (`rgb(255 255 255 /
0.04–0.08)`: the lit World slot, Settings' cycle and on/off controls) also blend
in linear light, so they come out much lighter than the mockup's (the lit New
World tile (83,84,83) against (19,22,22)). The title menu behind Settings still
shows faintly through the sheet, which the mockup hides.
