# The bulk solver and the propagation rules

Status: done
Blocked by: 01

The per-Chunk solve that streaming pays for on every tick. One contiguous
sweep, not a seeded BFS through a random-access interface - spec decision 6.

## What to build

In `light.rs`:

- `fn blocks_light(voxel: Voxel) -> bool`, from `Voxel::is_full_cube`. A shaped
  Voxel is conservatively transparent, matching the mesher's existing choice;
  per-Shape opacity is build-order step 5.
- `fn emission(voxel: Voxel) -> u8`. A `match` on the Material, not a dense
  table: a Material id is a `u16` and a dense table is 64 KiB of zeroes. Add
  `LANTERN` to `worldgen`'s placeholder Materials and give it `MAX_LIGHT`;
  generation places none of it.
- `fn transmitted(channel, level, face) -> u8` - the single rule both solvers
  share. Block light loses one level in every direction. Sky light loses one
  level in every direction **except** straight down at full strength, where it
  keeps `MAX_LIGHT`. That one exception is what makes open ground uniformly lit
  however tall the air column above it is, and it is the rule the decrease pass
  in ticket 03 has to be careful about.
- `pub fn light_chunk(center: &Chunk, border: [Option<&ChunkLight>; 6]) ->
  ChunkLight`. A padded 34-cubed light buffer seeded from `border`, the Chunk's
  own emitters and the sky plane, then a BFS to a fixed point over the interior.
  `None` on a face is a dark boundary, not an air boundary.

No neighbour *geometry* parameter. An opaque Voxel stores light 0, so the
dimming rule is already blocked by an opaque neighbour - the opacity is encoded
in the light the neighbour carries. Say so in a comment; its absence looks like
an oversight.

## Tests

- A Chunk of air under a uniform sky-15 +Y boundary is uniformly sky 15: the
  undimmed descent, over the full 32 Voxels.
- A solid floor under that air: the floor's own Voxels are 0 and the air above
  is 15.
- Light entering one face dims by exactly one level per Voxel and reaches
  exactly 15 Voxels, not 16.
- A single emitter in the middle of an air Chunk gives a correct Chebyshev-free
  (Manhattan) falloff, and 0 beyond 15.
- An emitter behind a solid wall does not light the far side.
- All six `None` faces and no emitter gives a uniformly dark Chunk, and it stays
  the `Uniform` variant.
- A uniform solid Chunk solves to uniformly dark without going dense.
