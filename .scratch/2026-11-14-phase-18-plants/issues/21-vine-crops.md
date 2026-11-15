# 21: Vine crops: watermelon and pumpkin

Status: done

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

- [x] A vine bears fruit up to `max-fruit`, never touching (the gap in Voxels,
      at least one) and within reach, at 1.0, 0.5 and 0.25 m.
- [x] Fruit ripens through its Stages and does not seed.
- [x] Breaking a fruit lets the vine bear another (Server test).
- [x] Fruit is no Item's Material and cannot be placed; breaking ripe fruit
      drops its yield.
- [x] A vine and its first fruit generate whole across a Chunk border and
      identically for equal seeds; at every Voxel size the count, gap and reach
      hold.
- [x] The snapshot round trip carries a generated vine; an older save loads.
- [x] Docs and the ADR-0027 amendment record the rules.

## Comments

Built. `crop-def.fruit-vine: option<fruit-vine-def>` (WIT in both copies). Fruit
reuses `Role::Fruit` for a crop Plant: grown Materials, no implicit Item, soil
below like a crop (`lacks_soil` now holds only a tree's own parts back), yields
by Stage, `canonical` and `stage_shape` for undo and Shapes, and `is_tree_part`
now asks the Plant's kind so ground fruit is never Felling. The ripe vine Stage
Samples at `regrow-seconds`; `reach::bear_vine_fruit` picks one hash cell in
reach (the spread picker), needs it free with `needs_met` (soil, light, Climate),
fewer than `max-fruit` of the Plant's fruit within reach and none within
`gap_voxels` (`ceil(gap / size)`, at least 1) on the ground plane; fruit Samples
to ripen through its Stages and nothing else. A vine crop with `harvest-to` is a
load error.

Worldgen. A new `SurfacePlantKind::Vine` (frozen in the Biome, `SavedPlantKind::
Vine`, optional `vine` form) stands the ripe vine and `first_fruit_offsets`
fruit: candidate columns in reach ranked by a hash of the seed and the vine's
column in metres, accepted greedily when apart and when `ground_base` (the checks
of `plant_at` without the Biome: above the terrain band, not carved, not under
the lake, not eased) holds, one to `max-fruit` of them. Cells are put after every
tree, so a trunk or crown is never cut, from any Chunk's viewpoint; the feature
margin is now the widest of tree and vine reach. A first fruit is a function of
the vine's metres, but which cells are candidates depends on the Voxel size, so
the cluster is not identical at every size, only whole and within the rules.

Content. `watermelon` and `pumpkin` in `crops` (box Shapes, five tiles and an
item icon each from `scripts/generate-mod-art`, a seed Item, slices and a pumpkin
as food), scattered in Terra's temperate and savanna, meadow's meadow, prairie
and heath, tidewater's lagoon, lumen's garden and ember's ash flats, each under
the 16-feature cap. The Mod's description stays under 80 characters.

Tests: core `plant::reach` (bears within reach, never touching, to the cap, at
three sizes; breaking lets it bear another; fruit alone never changes more than
itself; first fruit offsets deterministic and apart), `plant` (grown, yields,
soil, load rules), `plant_worldgen` (whole across Chunk borders at three sizes,
order independent, the snapshot carries it), mods (resolve, defaults, freeze,
errors, TOML), server (`fluids::plants::tests::vine`: cap, replacement after a
break, fruit alone ripens and seeds nothing, fruit breaks without ground) and
`demo_games` (content rows, yields, no Item places fruit).

Re-pinned: `the_demo_games_textures_are_unchanged`; the per-Biome species tables
of `demo_games::vegetation`. No version change: the snapshot kind is additive.
