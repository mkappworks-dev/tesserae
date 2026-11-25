# 07: Vibrance pass

Status: ready-for-agent

Spec: [Colour, art and light](../spec.md).

## What to build

The user's play-test of 01 to 06: less saturated, but the look turned "dusty"
and "rusty"; the leaves are not vibrant, and neither is the dirt ground. Three
of the spec's decisions point that way: grass, Terra's leaves and the
broadleaf forests were moved toward the acacia's chroma (C\* 35 to 40,
slightly darker), so every forest took a savanna olive; dirt went to S 0.40
toward yellow-brown; and the lowered sun, the haze from 30 m and the 50 to 243
texel range flatten what is left.

1. Attribution (Q83): one comparison sheet at the village and jungle Eyes, 70
   and 110 m, and a grass field, in four columns: `main`; this branch; this
   branch with `main`'s art; this branch with the haze off. The user picks the
   column closest to the look they want.
2. Retune (Q84), from what step 1 shows, starting points the user approves:
   - Grass top, Terra's leaves and `flora`'s oak, jungle, cherry, orange and
     apple leaves to C\* about 45 to 50, hue kept green (not toward yellow),
     no darkening. Acacia and pine stay.
   - Dirt to S about 0.45, hue toward a neutral to red brown, a little
     brighter.
   - Light or haze only if step 1 points at them.
   - Pattern rules and the 50 to 243 texel range stay. Art only through
     `scripts/generate-mod-art` and Terra's `textures.rs`; appearance colours
     recomputed by the same rules (block tile mean, sprite edge colour).

## Acceptance

- [x] The step 1 sheet, and the user's pick, recorded here.
- [x] Leaves and grass read vibrant green, dirt reads as soil, dry Biomes still
      read dry; approved on near, 70 m and 110 m captures.
- [ ] Generator re-run byte-identical; appearance-colour tests pass.
- [x] Spec's colour targets updated to the approved values.

## Comments

2026-11-25: the Q83 sheet, the pick, and Q86 to Q91.

Captures in `/tmp/colour-07/sheet/` (never committed), offscreen 1920x1080,
level 1 settled to 6743 Chunks, `--release --features dev,profiling`. Columns:
`main` (0b03c594, built from an archive with `dev`, its own Terra and Mods);
this branch (8f2522d8); this branch with `main`'s art (`main`'s Mod PNGs and
appearance colours, canopy variants re-derived from `main`'s leaves by the
generator's rule, Terra's `textures.rs` and `building.rs` and its appearance
bases from `main`; this branch's sky, fog, light and patterns kept); this
branch with the haze off. Views: village and jungle at 70 and 110 m, a grass
field at eye height (`1700.37,-722.41`, yaw 0, pitch 3) and the `near` grass.

| On screen               | `main`   | branch   | + art    | haze off |
| ----------------------- | -------- | -------- | -------- | -------- |
| Near grass C\*, L\*     | 50.7, 69 | 32.2, 60 | 47.5, 61 | 32.4, 60 |
| Jungle 70 m leaves C\*  | 25.7     | 21.6     | 28.7     | 24.4     |
| Jungle 70 m dirt C\*, h | 15.8, 67 | 13.1, 78 | 17.2, 66 | 13.2, 78 |
| Step top / side, linear | 3.2      | 1.7      | 1.6      |          |

- The art is the cause: `main`'s art under this branch's light and sky gets
  the green chroma back. The haze changes nothing visible at 70 or 110 m. The
  light darkens (about 8 L\*) and flattens: a sunlit top against a shaded
  step side fell from 3.2 to 1.6, since 02's sun 10 000 -> 3600 lux and sky
  900 -> 1500.
- Q86: the user picked "branch + `main`'s art". The retune aims at its
  greens; dirt between `main`'s salmon and this branch's grey-yellow.
- Q89: the user wants the shading between block layers back, but cast
  shadows of trees and the look of grass tufts and plants kept as soft as
  now. A stronger sun darkens both alike, so instead the sky's ambient share
  goes by a face's direction: top 1, side about 0.6, underside about 0.4,
  crosses exempt, the sun and cast shadows untouched. Captured at side 0.6
  and 0.7 against today's, with `main`'s art.
- Q90: the light first, then the art retune judged under it.
- Q91: the sky share applies at every level, so no seam at a level's join.
- Q88: the haze stays.

2026-11-25: the sky share and the retune, approved by the user (Q92 to Q98).

Captures in `/tmp/colour-07/light/` and `/tmp/colour-07/sheet/retune3/`, same
recipe as the sheet, every view at level 1's 6743 Chunks. Columns against the
user's pick ("branch + `main`'s art") at the same light.

- Sky share (Q92): a side at 0.6, 0.45 and 0.35 was captured against today's.
  The tonemapper's toe softens it: 0.6 gave a step top / side of 2.0, not the
  2.7 the light alone predicts. 0.45 is the pick: steps and walls read layered
  (2.3), a trunk under the jungle canopy dims from 0.151 to 0.076 linear and
  still reads, the jungle floor 0.139 -> 0.115. Underside 0.4; a cross keeps
  the whole sky; a cave's faint floor (`UNLIT_AMBIENT`) is untouched.
- Greens (Q93, Q94): the leaves palettes x1.38 in C\*, hue +6 degrees,
  lightness kept (tile C\* about 50, `main`'s about 51 to 56). Terra's grass and
  leaves painters drew hard-coded colours and ignored their base; they now
  paint from it. The generator's `toward_acacia` (C\* x0.667, 5% darker, hue 6
  degrees yellower) is replaced by `green_target` (C\* x0.85, lightness and
  hue kept) on the ground cover. On screen at the same light: grass field
  greens C\* 39.8 against the pick's 43.4, jungle leaves 21.9 against 23.5.
- Dirt (Q95, Q97): `[109, 81, 60]`, `spread` 7 -> 5 and pebbles over 216
  instead of 236. The first retune's brighter dirt (0.204, texel contrast
  0.20) read as a veil; now 0.183 and 0.24 against the pick's 0.184 and 0.29.
- Bark (Q96, Q97): the furrows' sideways step was regular, so every block of
  a trunk drew the same staircase; it is now a random step per 4-row band,
  with 1x2 scars (1 in 16) and 1x2 moss at half the count.
- Dry Biomes still read dry on `dry-ground`, `savanna-ground`, `savanna-far`.
- `flora`'s appearance colours set by the rules (block tile mean, sprite edge
  colour; the measure reproduces poppy, daisy and orchid exactly). A second
  generator run is byte-identical over all 166 PNGs. `flora` and `crops`
  thumbnails regenerated.

Tests written, not run: the sky share's shader defs in
`render/chunk_material.rs`, and a compile-time check that an underside sees no
more sky than a side and a side less than a top.
