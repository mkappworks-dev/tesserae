# A coarse lake bed is dimmed by the water over it

Status: done
Blocked by: 09

Coarse levels store no light, and main's covered-column rule (#15) darkens only
Air under the ground. So under a coarse lake the bed's faces look out at Water
lit as open sky, while level 0's bed is dimmed by 2 for each Voxel of water
above it. Where a finer level's water lies over a coarse bed, at every ring
join, the bed shows bright through it.

## What to build

- `coarse_light` gives a Water Voxel the sky light level 0 would give the
  level-0 Voxel at its bottom: 15 less the water's dimming for every level-0
  Voxel of water above that point, never below 0. Depth comes from the water
  level, which the generator knows, and is counted in level-0 Voxels because a
  light level is a count of steps, not a distance.
- A dry World's coarse light is unchanged.

## Acceptance criteria

- Tests: a Water Voxel at a given depth gets what the level-0 flood gives the
  same depth; Air above the water is still open sky; a dry World's light is
  unchanged.
- Capture of a ring join over a lake: the bed through the finer water is no
  brighter than level 0's.
- The `lod` benches are unchanged against `main` for a dry World.
- `./scripts/cargo-ci` passes.

## Comments

Built as specified, with the formula corrected by its own test.

- `coarse_light_in` takes an `UnderWater` (the water level, Water's dimming, and
  the level-0 Voxel's edge); `coarse_light` is the dry form the existing tests
  and the bench call. The Server builds it per level in `mesh_coarse`, the
  level-0 Voxel being the coarse one over `2^level`.
- The first formula took 2 levels per level-0 Voxel. The test asks level 0's own
  flood for the truth, and it disagreed below the top Voxel: under full
  strength, sky light no longer falls free, so each Voxel after the first costs
  a step as well as the dimming (13, 10, 7, 4, 1, 0). The formula now matches it
  at every coarse row.
- Three Server tests asserted that every coarse vertex is open sky or covered.
  Two are about the covered-column rule and now build a dry World; the third
  now accepts the depth levels as well, and still no block light.
- Capture, the same ring join as ticket 07's: water luminance across the band is
  133 to 150 with no row-to-row step above 5.9, against 131.5 to 154 and 10.1
  before. A dry World's coarse light is unchanged (the existing tests).

- `lod/coarse_light` against `origin/main`, one codegen unit, three
  interleaved rounds: the five surface cases are unchanged (62.0 against 62.4
  us at level 1 to 106.4 against 108.4 at level 5), and `above_band_level_1`,
  a Chunk answered without walking it, is 17.8 ns against 14.1: the dry
  wrapper and the water check, a few microseconds over a whole ring.
