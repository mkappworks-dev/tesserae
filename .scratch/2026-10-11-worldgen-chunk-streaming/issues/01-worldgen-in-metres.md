# World generation, tuned in metres

Status: done
Blocked by: none

`tesserae_core::testgen` is a bench fixture, not a generator: its noise
frequency is `1.0 / 48.0` per Voxel index and its sea level and relief are bare
Voxel counts. ADR-0003 names tuning a noise frequency in Voxel units as the one
decision that cannot be repaired later. This ticket adds the real generator,
with every spatial parameter in metres.

## Task

Add `crates/tesserae-core/src/noise.rs` and
`crates/tesserae-core/src/worldgen.rs`.

`noise.rs` holds the value-noise primitive: the integer mix, the lattice
sample, the smoothstep-interpolated `noise2`, and a normalised fractal sum.
Move the existing functions out of `testgen.rs` verbatim so the fixture's
output does not change — `testgen`'s determinism test pins it, and the mesher
benches must stay comparable to step 2's numbers. `testgen` then calls into
`noise`.

`worldgen.rs` holds:

- `WorldgenParams`, plain `Copy` data with a `Default`. Every spatial field
  carries a `_m` suffix and is in metres: sea level, relief, the horizontal
  feature size of the lowest octave, and the soil depth. Octave count is
  dimensionless. Shaped so step 7's WIT contract can fill it; no callbacks, no
  trait objects.
- The placeholder Material ids. They live here rather than in `testgen` because
  worldgen is what assigns them; a Mod owns this table from step 7.
  Re-export them from `testgen` so `testgen::STONE` keeps resolving.
- `Worldgen`, built from a seed, the World's `voxel_size` and a
  `WorldgenParams`.
- `Worldgen::surface_height_m(x_m, z_m) -> f32`.
- `Worldgen::chunk(cx, cy, cz) -> Chunk`.

`chunk` converts each Voxel index to metres through `voxel_size`, once, and
every comparison after that is between metres. The topmost solid Voxel is
grass, Voxels within `soil_depth_m` of the surface are dirt, the rest is stone —
all three expressed as metre distances below the surface, so the soil layer is
the same physical thickness at any Voxel size.

### The two early-outs

A normalised fractal sum lands in `[0, 1]`, so the surface is bounded between
`sea_level_m` and `sea_level_m + relief_m`. Before sampling any noise:

- a Chunk whose lowest Voxel is above that band is `Chunk::air()`
- a Chunk whose highest Voxel is below the band by more than `soil_depth_m` is
  `Chunk::filled` with stone

Most Chunks in a 3D ring are one or the other, so this is what makes a vertical
extent affordable at all rather than a micro-optimisation. Comment it as such.

## Tests

- **The invariant test.** Generate the same seed at `voxel_size` 1.0 and 0.5
  and assert `surface_height_m` agrees for the same metre coordinates, and that
  the highest solid Voxel in a column lands at the same metre height within one
  Voxel. This is the test that would fail if a frequency were ever retuned in
  Voxel units.
- Determinism: the same seed and coordinates give the same Chunk twice.
- A different seed gives different terrain.
- Both early-outs fire: assert `uniform()` is `Some(air)` far above the band
  and `Some(stone)` far below it, and that a Chunk straddling the surface has
  neither.
- `testgen::terrain_chunk` output is byte-identical to before the noise move.
  Easiest as a hash or a spot-check of known indices; the existing determinism
  test does not catch a changed-but-still-deterministic generator.

## Bench

Add a `worldgen` group to `crates/tesserae-core/benches/meshing.rs`: a Chunk
straddling the surface, a Chunk in the air early-out, and a Chunk in the stone
early-out. Rename the existing `worldgen_chunk` bench to name the fixture it
actually measures — it benches `testgen::terrain_chunk` — and say in the PR
that the id changed so the before/after comparison stays honest.

## Done when

`cargo test -p tesserae-core` and `cargo bench -p tesserae-core` pass, no new
dependency appears in `Cargo.lock`, and the mesher bench numbers are unchanged
against step 2's baseline.

## Comments
