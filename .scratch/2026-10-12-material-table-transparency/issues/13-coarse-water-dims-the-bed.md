# A coarse lake bed is dimmed by the water over it

Status: ready-for-agent
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
