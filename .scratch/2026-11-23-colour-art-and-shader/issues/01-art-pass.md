# 01: Art pass

Status: done

Spec: [Colour, art and light](../spec.md).

## What to build

Solution item 1, in this order, each step shown on captures the user approves
before the next:

1. Pattern rules from the spec's Decisions in `games/terra/src/textures.rs` and
   `building.rs`: periods that divide 16 (leaves first), clumped speckles, two
   noise octaves, a scaling `tint`, the 50 to 243 texel range, thicker sprites.
2. The same rules, by hand, in the PNGs of `flora` (leaves, logs, ground
   cover), `strata` (rock), `timber` (planks, thatch), `brickwork` (brick,
   cobble, roof tile) and `crops` (wheat: thicker stalks).
3. Colour targets: grass and leaves, dirt, sand and snow, `flora:cane`, and
   every Biome's sky (blue overhead) and fog colour (its character).
4. Appearance colours set to each tile's linear mean.

## Acceptance

- [ ] Every period-bound pattern in Terra tiles across a Voxel edge (test).
- [ ] Terra's appearance colours equal their tile's linear mean (test).
- [ ] Terra's existing texture tests pass.
- [ ] Each step approved on near, mid (70 and 110 m) and far captures, against
      the baseline.
- [ ] No sunlit sand or snow top above about 85% value on `dry-ground` and
      `cold-ground`.
- [ ] Wheat keeps its shape at distance; approved on a crop field capture.

## Comments

2026-11-25: review findings and Q73, Q75.

- The twelve `_dark` and `_light` canopy variants now come from
  `scripts/generate-mod-art` (`variant`: HSV value times 0.92 or 1.08, hue
  4 degrees, alpha kept, then `albedo()`). The light ones are pixel for pixel
  as before; the dark ones are lifted to luminance 50 (they reached 46).
- Flowers, saplings and the box-drawn fruit, mushrooms and berry bushes scale
  in linear light (`shade`) and pass through `albedo()`: daisy no longer
  reaches 248, `cherry_ripe` no longer 43. `crops`' melon, pumpkin and berry
  bush share those painters, so their tiles moved a little too.
- A re-run of the generator leaves `git status` clean.
- Q75: a sprite (a Material a Plant places on a cross or plane Shape) takes
  its edge colour, the linear mean of its drawn texels with a clear texel
  left, right, above or below on the same face. Every sprite in every Mod is
  set by it. Moved more than 10 levels: `flora:poppy` [165, 114, 43] to
  [133, 104, 42], `daisy` [183, 189, 158] to [153, 172, 142], `dandelion`
  [188, 176, 57] to [156, 155, 42], `orchid` [137, 129, 163] to
  [104, 112, 130]. Berry bush Stages are box Shapes, so they keep the tile
  mean.
- Q73: crops keep the tile mean; their sprites follow Q75.
- `terra_appearance_colours_are_a_blocks_tile_mean_and_a_sprites_edge_colour`
  checks Terra, the blocks of `flora`, `strata` and `crops`, and every
  sprite, to within one level of the unrounded measure; fourteen colours were
  set one level closer for it.
