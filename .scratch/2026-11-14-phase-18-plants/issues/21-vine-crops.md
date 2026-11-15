# 21: Vine crops: watermelon and pumpkin

Status: ready-for-agent

Blocked by: None (the mockup is approved)

Spec: [Phase 18](../spec.md), amendment 2026-11-15.

## What to build

A `crop` may declare `fruit-vine`: fruit its ripe Stage bears on the ground
around it, as a tree bears fruit on its canopy.

```
record fruit-vine-def {
    stages: list<plant-stage>,   // 2 to 4, the last ripe, like a tree's fruit
    regrow-seconds: f32,         // at least 16
    shape: option<string>,       // the fruit's Shape, as a tree's fruit
    reach-m: option<f32>,        // [1.5], above 0, at most 8
    gap-m: option<f32>,          // [0.5], 0 to reach-m
    max-fruit: option<u32>,      // [5], 1 to 16
}
```

- The vine's ripe (last) Stage Voxel Samples at a mean of `regrow-seconds`. On
  a pass it bears one unripe fruit when fewer than `max-fruit` of its fruit
  stand within `reach-m`, on a hash-picked free Air cell in reach that has the
  Plant's soil below and passes the Plant's light and Climate there, and no
  fruit of the Plant lies within `ceil(gap-m / voxel size)` (at least 1) Voxels.
  Fruit does not seed: it only ripens through its Stages.
- Fruit Materials are grown: no implicit Item, never placeable. Breaking a fruit
  is an ordinary journalled removal that drops that Stage's yields; the vine
  bears another. Fruit needs soil below like a crop, and breaks without it.
- A crop with `fruit-vine` has no `harvest-to`.
- Resolver: fruit Stages resolve like a tree's; the Material rules (one Plant
  each) apply.
- Worldgen: a Biome feature naming a vine crop stands the ripe vine with its
  first fruits, a pure function of the seed and the vine's column in metres,
  whole across Chunk borders. The Biome snapshot freezes it as an optional
  `vine` kind, so no version changes.
- Content: Watermelon and Pumpkin in the `crops` Mod (box Shapes, tiles and item
  icons from `scripts/generate-mod-art`, a seed Item and a food Item each), and
  in the Terra, meadow, tidewater, lumen and ember Biome features where they
  make sense, within the 16-feature cap.

## Acceptance criteria

- [ ] A vine bears fruit up to `max-fruit`, never touching (the gap in Voxels,
      at least one) and within reach, at 1.0, 0.5 and 0.25 m.
- [ ] Fruit ripens through its Stages and does not seed.
- [ ] Breaking a fruit lets the vine bear another (Server test).
- [ ] Fruit is no Item's Material and cannot be placed; breaking ripe fruit
      drops its yield.
- [ ] A vine and its first fruit generate whole across a Chunk border and
      identically for equal seeds; at every Voxel size the count, gap and reach
      hold.
- [ ] The snapshot round trip carries a generated vine; an older save loads.
- [ ] Docs and the ADR-0027 amendment record the rules.

## Comments
