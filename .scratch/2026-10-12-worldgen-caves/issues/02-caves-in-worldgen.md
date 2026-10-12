# Carve tunnels in `Worldgen::chunk`, keeping the deep early-out

Status: ready-for-agent
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
