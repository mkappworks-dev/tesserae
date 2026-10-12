# Carve tunnels in `Worldgen::chunk`, keeping the deep early-out

Status: done
Blocked by: 01

Spec decisions 1 to 5.

## What to build

- `CaveParams { feature_size_m, radius_m, floor_m, octaves }` and
  `WorldgenParams::caves: Option<CaveParams>`, default on. Re-exported from the
  crate root with `WorldgenParams`.
- A private `CaveField`: two seeded `fbm3` fields, `carves(x, y, z)` and
  `may_carve(lo, hi)`. `may_carve` is the `fbm3_range` test against the window;
  `false` is a proof.
- `Worldgen::chunk`: the three layers of spec decision 2. Below `floor_m`, or
  when `may_carve` is false for the Chunk's box, answer as today. A deep Chunk
  that may carve is `Chunk::filled(STONE)` then carved in the 8-Voxel sub-blocks
  `may_carve` cannot rule out. A surface Chunk skips carving in the same
  sub-blocks. A mouth removes the Grass cap.
- Nothing above `surface_bounds_m().1` changes. Say so in the `surface_bounds_m`
  doc, which the Server's open-sky boundary leans on.
- Remove no existing test silently; change them as the spec says and say why in
  a comment only where the reason is not obvious.

## Acceptance criteria

- A slow reference in the tests (every Voxel, `surface_height_m` and `carves`
  directly, no culling) equals `chunk()` over a sweep of Chunks at two Voxel
  sizes, so no layer of the early-out changes an output.
- The same metre point is carved or not at 1.0, 0.5 and 0.25 m Voxels.
- With `caves: None`, every existing worldgen test holds unchanged, including
  exactly one Grass Voxel per column.
- With caves on: at most one Grass Voxel per column and it is the highest solid;
  the surface stays inside the bounds; a sweep of Chunks wholly above the band
  is all air; Dirt and Stone both appear in a cave wall in some Chunk.
- `both_early_outs_fire_and_a_straddling_chunk_takes_neither` is split: the
  sky and below-`floor_m` Chunks are uniform; a deep Chunk inside the band is
  uniform stone or carved, and the test finds one of each.
- `testgen` is untouched; `terrain_output_is_pinned` passes.
- `./scripts/cargo-ci` passes.

## Comments

**What was built.** `CaveParams` (re-exported at the crate root),
`WorldgenParams::caves: Option<CaveParams>` defaulting on, and a private
`CaveField`. `Worldgen::chunk` has the three layers of the spec: the floor and
sky answers as before; a deep Chunk is `Chunk::filled(STONE)` carved in the
8-Voxel sub-blocks the range test cannot exclude, so a Chunk that passes the
test and carves nothing stays `uniform()`; a surface Chunk computes its 1024
column heights first and clamps the test to the tallest, because nothing above
it is solid.

**Tests.** The terrain tests run on a `terrain()` generator with caves off and
are otherwise unchanged. New: caves only ever remove Voxels (the structural
reason nothing is solid above the band), the slow every-Voxel reference equals
`chunk()` at 1.0 and 0.5 m Voxels over a 96 m square sweep, the same metre
points are carved at 1.0 and 0.5 m, a cave takes the same share of the ground at
1.0 and 0.25 m, nothing below the floor is carved, a deep world with sparser
tunnels keeps more Chunks uniform than it carves, at most one Grass per column
and it is on top, and a mouth exists with Dirt and Stone in its walls.

**Decision: a mouth removes the Grass cap.** The "exactly one" test now runs on
the caveless generator, and the caved one asserts "at most one, and highest".

**Measured, and the spec corrected for it.** The first prototype numbers said
82% of deep Chunks stayed uniform; that counted layers under the floor. Per
layer in the band the range test keeps 27% (64 m features) to 76% (128 m)
uniform, so the floor is the lever and the spec says so.
