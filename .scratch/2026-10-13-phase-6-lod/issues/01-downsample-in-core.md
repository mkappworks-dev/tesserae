# The downsample in core

Status: ready-for-agent
Blocked by: none

Spec decision 2. The pure function the whole step rests on.

## What to build

- `crates/tesserae-core/src/lod.rs`, exported from `lib.rs`:
  `pub fn downsample(children: [&Chunk; 8], shapes: &ShapeTable) -> Chunk`.
  Child `i` is octant `x + 2y + 4z`; coarse Voxel `(cx, cy, cz)` covers
  `(2cx + dx, 2cy + dy, 2cz + dz)` of the 64^3 lattice the children form.
- Weight of a child Voxel is `shapes.volume(v)` (air 0 whatever its shape byte).
  Coarse Voxel is solid iff `2 * sum >= 8 * 4096`; a solid result is always
  `Voxel::cube(material)`.
- Material is the largest per-Material weight total over the eight children.
  Ties: the Material whose highest child (largest `dy`) is higher; then the
  lower `MaterialId`.
- Fast paths: eight identical uniform children that pass the test return
  `Chunk::filled`; eight air children return air. Everything else takes the
  general loop. Write the plain loop first, benchmark it, and only then tighten
  it, comparing each tightening against the plain version.
- `crates/tesserae-core/benches/lod.rs` (new, with a `[[bench]]` entry,
  `harness = false`): `lod/downsample` over terrain children, uniform children
  and stair-heavy children. Do not edit `benches/meshing.rs`.
- No dependency is added to core. No `rayon`.

## Tests

- The half-volume rule against the three built-in Shapes: a slab (2048) and a
  stair (3072) read as solid, a post as air, per the phase-5 spec decision 10; a
  tie (exactly half) is solid, one sub-cell under is air.
- Grass over dirt at every surface height: the four-to-four tie keeps GRASS, and
  a test fails if rule 1 of the tie-break is removed (mutation-check it).
- Tie-break independent of palette order and of child order for equal top
  heights (lowest id).
- Translucent Materials (glass, leaves, water) keep their id and count as
  occupied; a one-Voxel pane in an air Chunk vanishes.
- A reference implementation in the test (a brute-force 64^3 lattice) agrees
  with `downsample` over seeded random children, including shaped ones in all 24
  Orientations.
- Output never contains a shaped Voxel.
- Fast paths agree with the general loop.

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- `cargo bench -p tesserae-core --bench lod` runs; numbers (and the load
  average) in Comments.
- No existing benchmark source is edited.

## Comments
