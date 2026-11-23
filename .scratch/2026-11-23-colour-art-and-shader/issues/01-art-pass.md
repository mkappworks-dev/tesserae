# 01: Art pass

Status: ready-for-agent

Spec: [Colour, art and light](../spec.md).

## What to build

Solution item 1, in this order, each step shown on captures the user approves
before the next:

1. Pattern rules from the spec's Decisions in `games/terra/src/textures.rs` and
   `building.rs`: periods that divide 16 (leaves first), clumped speckles, two
   noise octaves, a scaling `tint`, the 50 to 243 texel range, thicker sprites.
2. The same rules, by hand, in the PNGs of `flora` (leaves, logs, ground
   cover), `strata` (rock), `timber` (planks, thatch) and `brickwork` (brick,
   cobble, roof tile).
3. Colour targets: grass and leaves, dirt, sand and snow, `flora:cane`, and
   every Biome's sky and fog colour.
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
