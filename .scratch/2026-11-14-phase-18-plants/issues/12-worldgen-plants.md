# 12: Plants in worldgen

Status: done

Blocked by: 02, 07, 08, 09, 10

Spec: [Phase 18](../spec.md).

## What to build

A Biome's `surface-features` can name a Plant. Generation places crops, ground
cover and vines at a hash-drawn Stage and full-grown trees, under the existing
feature rules (a full cube of the surface Material, Air above, inside the global
surface bound, never under a fluid or on a stair or slab or in a cave mouth); a
kelp `column` stands in the lake. A tree is a pure function of the seed and a
column position in metres, so each Chunk generates the Voxels of neighbouring
columns that fall inside it, scanning the Chunks within `canopy-radius-m`. Level
0 only; no new Generator version.

## Acceptance criteria

- [x] A tree across a Chunk border is whole, and equal at 1.0, 0.5 and 0.25 m
      in metres, in a test that generates both neighbours in either order.
- [x] A generated fruit tree has fruit at hash-drawn Stages inside its density,
      the same whichever Chunk generates a Voxel of it.
- [x] A version 2 World whose Biomes name no Plant generates the digests
      recorded before Plants existed.
- [x] Plant rows ride in the World's Biome snapshot; a World saved before
      Plants keeps its bare ground.
- [x] Coarse levels generate without Plants and do not vote on them.
- [x] A Chunk's generation time with trees does not exceed `main`'s by more
      than the bench's agreed margin.

## Comments

- A feature is now `material` or `plant` (WIT `option`s), plus `shape`: a
  Plant's Material carries no Shape (a Player's Place chose it), so a Biome
  names the Shape its crop, spreader or column Voxels are generated with; none
  is a cube, a tree is always cubes. Load errors: both or neither set, an
  unknown Plant or Shape, a `climber` (needs a wall, not generated), a column
  growing `down`, a `shape` on a tree or a Material.
- `SurfaceFeature` gains `plant: Option<SurfacePlant>`, an owned `Copy` copy of
  what generation reads (Stages, a column's Material, height and fluid, a
  tree's Materials, height, radius and fruit), so `WorldgenParams` stays
  `Copy`. The snapshot writes it by Material and Shape name under
  `plant: {shape?, form: {kind, ..}}`, absent for a Material feature; no Plant
  name is frozen, nothing reads it. Formats and Generator versions are
  unchanged.
- A Plant column is decided by `Worldgen::plant_at`, a pure function of the
  seed and the column in metres that reads no Chunk (cave carving and smoothing
  are asked by metre position, as `still_solid` does), so every Chunk within
  `canopy-radius-m` agrees. The tree is grown at height 0 and shifted onto the
  ground, so it is the same in metres whichever Voxel the ground rounds to.
  Trunks claim their cells before any canopy; otherwise the first column in
  scan order wins, and only Air is replaced.
- `surface_bounds_m().1` rises by the tallest tree or column a Biome declares,
  so streaming extent and the open-sky shortcut stay sound; a World with no
  Plant feature is bounded and generated exactly as before. Ores and ground
  features keep the unraised band.
- Kelp replaces the lake's water cube with its shaped Voxel; whether the Server
  refills the cell as waterlogged is not exercised here.
- Climbers are not generated. A crop is placed at a hash-drawn Stage of any
  index, ripe included.
- `resolve_biomes` now runs after the Plant table (loader reordered through
  `biome_rows`), and `Plant::name` is the full `<mod id>:<name>`.
- Generation-time margin, measured by the benchmark pass (Apple M5, thread CPU
  time, interleaved, load 8 to 9; `cargo bench -p tesserae-core --bench plants`):
  a surface Chunk of a Biome with 16 features (3 trees, 13 plants, spacing 4 to
  13 m) takes 0.42, 0.24 and 0.20 ms at 1.0, 0.5 and 0.25 m, against 0.12, 0.08
  and 0.08 ms bare, so at most 3.7 times and 0.31 ms more. The margin is that.
  The trees alone are 1.4 to 2.1 times, the 13 plants alone 1.8 to 2.6 times.
  Air and stone Chunks keep their early-outs.
