# 08: Art for the new shared building Materials

Status: done

Blocked by: 03, and `fix/colour-art-and-shader` merged to main

Spec: [Shared building Materials](../spec.md).

## What to build

The Mod art generator draws every Material added in tickets 02 and 03 that has
no art: `timber`'s `pine_planks`, `dark_planks`, `red_planks`, `frame`,
`dark_frame`, `shingles`, `awning`, `door` and `gate` (closed and open);
`brickwork`'s `ashlar`, `slate`, `red_tile` and `whitewash`; `workshop`'s
`workbench`, `forge`, `anvil`, `oven` and `chest`. Each is drawn in the colour
session's style, with Terra's look for the Material it replaces as the target,
and its appearance colour is its tile's linear mean, as that session's rule
requires. The art the colour session drew is not redrawn.

## Acceptance criteria

- [ ] Every shared building Material has a texture from the generator, and the
      PNGs match a fresh run of it.
- [ ] Each appearance colour is its tile's linear mean.
- [ ] A Terra village at a fixed seed reads as it did before ticket 04, in a dev
      capture compared by eye (review-only, not committed).
