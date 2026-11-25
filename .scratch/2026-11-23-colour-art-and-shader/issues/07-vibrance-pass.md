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

- [ ] The step 1 sheet, and the user's pick, recorded here.
- [ ] Leaves and grass read vibrant green, dirt reads as soil, dry Biomes still
      read dry; approved on near, 70 m and 110 m captures.
- [ ] Generator re-run byte-identical; appearance-colour tests pass.
- [ ] Spec's colour targets updated to the approved values.

## Comments
