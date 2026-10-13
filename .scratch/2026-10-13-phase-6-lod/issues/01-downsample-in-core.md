# The downsample in core

Status: done
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

Ten unit tests in `lod.rs`; the brute-force reference is a 64^3 lattice over 12
seeded children (2x2x2 blocks of noise, two Materials in halves, or one with a
stranger, drawn from air, six cubes and slab, stair, post and fence in all 24
Orientations). Mutation checks, each restored afterwards: dropping the height
term from the tie-break fails the grass-over-dirt test, the height test and the
reference test; making the id rule prefer the higher id fails the lowest-id test
and the reference test.

Benchmarks (`cargo bench -p tesserae-core --bench lod`, Apple M-series, load
average 4.0 / 10.5 / 11.1 before and 3.1 / 9.5 / 10.7 after; the 1-minute figure
was 2.8-4.8 for every earlier run, the 15-minute tail is `cargo-ci`):

| `lod/downsample/`            | plain loop | shipped |
| ---------------------------- | ---------- | ------- |
| `worldgen` (surface-crossing) | 353 us     | 257 us  |
| `testgen`                    | 385 us     | 291 us  |
| `stair_surface`              | 417 us     | 333 us  |
| `stairs` (all 24 Orient.)    | 586 us     | 607 us  |
| `uniform_halves` (4 + 4)     | 264 us     | 33 us   |
| `uniform_stone` / `air`      | 3.3 ns     | 3.3 ns  |

A surface-crossing coarse Chunk is 257 us, well under the ~1 ms budget, and the
worst case I could build (every Voxel a stair) is 0.6 ms. `worldgen` is the
default World's eight Chunks around the one the ground crosses, caves and
stairs included; half of them are uniform rock or air.

Decisions that departed from the ticket:

- **One tightening, measured.** The plain loop read every child Voxel through
  `Chunk::get`. A child that is uniform is now resolved once and written
  directly, which is the "uniform halves" case that every streamed surface Chunk
  has four of. The all-stairs case, which has no uniform child, is 3% slower
  (607 vs 586 us); I read that as run-to-run noise since its loop body is
  unchanged but did not chase it. Further tightening (a cache of the weights per
  palette entry, a fill that skips `Chunk::set`'s palette scan) was not tried:
  the budget is met, and `chunk.rs` is off limits.
- **Zero-weight children are skipped** in the Material tally, including for the
  height rule: a shaped Voxel whose Shape has no volume never makes its Material
  "higher". The ticket does not say; it cannot arise with the built-in Shapes.
- **Fast paths.** The whole-Chunk path covers eight identical uniform children,
  solid or not, and returns `Chunk::filled`. A mixed set of uniform children
  takes the general loop, which handles each uniform child as above.
- `downsample` is also exported at the crate root (`tesserae_core::downsample`),
  and `lod` as a module.
