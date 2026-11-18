# 11: Clippy refuses platform maths

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 05.

## What to build

The workspace's clippy configuration disallows the `std` `f32` and `f64` methods
the spec lists as non-deterministic, with a reason pointing at ADR-0032, so a
new landform that reaches for `.sin()` fails the Lint job on Linux. Every crate
but `tesserae-core` allows them at crate level. Inside it, audio, entity motion,
Plant growth, the sky blend, unit test modules, the integration tests and the
benches allow them explicitly. Each generation site 12 will correct carries a
temporary allow whose reason names 12, so the lint lands green now and, once 12
removes those allows, proves nothing is left.

## Acceptance criteria

- [x] A scratch build shows the lint firing on method-call syntax (`x.sin()`,
      `x.powi(2)`) on `f32` and `f64`, naming ADR-0032; noted in a comment here
      before anything else (this check can run before 05).
- [x] Every use of a listed method sits under an explicit allow: permanent
      outside generation, temporary naming 12 inside it.
- [x] The Lint job (`--all-targets`) passes.

## Comments

Scratch build (clippy 0.1.100, 2026-11-09), a crate outside the repo whose
`clippy.toml` disallowed `f32::sin`, `f64::sin`, `f32::powi` and `f64::powi`:
`x.sin()` and `x.powi(2)` fired on both `f32` and `f64`, as did `f32::sin(x)`,
each printing the configured reason naming ADR-0032; an
`#[allow(clippy::disallowed_methods)]` silenced it. A crate nested below the
`clippy.toml` with its own `[workspace]` inherits it too, which is why the Games
and `demo-kit` carry the allow as well.

The 26 methods are disallowed on `f32` and `f64` in the root `clippy.toml`.

Permanent allows (`allow`, 25):

- Package level, `[lints.clippy]` in `Cargo.toml` so every target is covered:
  `tesserae`, `tesserae-mods`, and outside the workspace `demo-kit`, `ember`,
  `lumen`, `meadow` and `terra`. CI clippies only the workspace, and those
  outside it already carry unrelated warnings.
- `tesserae-core` modules: `audio`, `entity` (Dropped item motion).
- `tesserae-core` functions: `Climate::sky_fog_at` (the sky blend) and
  `plant::water_within` (Plant growth; generation calls only `tree_voxels` and
  `vine_voxels` from `plant`).
- `tesserae-core` unit test modules: `lod`, `plant::reach`, `plant::tree`,
  `worldgen`.
- `tesserae-core` integration tests: `coarse_generation`, `fields`,
  `plant_worldgen`, `settlement_worldgen`, `structure_worldgen`,
  `surface_features`, `terrain_v3`, `underground`; benches: `coarse_breakdown`,
  `lod`, `structures`.

Temporary allows (15), each an `#[expect]` whose reason names ticket 12, so
once 12 changes the maths an allow left behind fails clippy as an unfulfilled
expectation:

- `powi` to multiplication (7): `terrain.rs` `Base::formation_m`,
  `Base::island`, `Base::volcano`; `worldgen.rs` `SurfaceScatter::new`,
  `chance`, `crowded`; `drainage.rs` `wet`.
- `sin`, `cos`, `atan2` to `libm` (8): `worldgen.rs` `dry_land_near_m`;
  `worldgen/fields.rs` `field_accepts`; `worldgen/settlements.rs`
  `settlement_spawn_m`, `near_spawn_draw`, `level_disc`, `lay_outskirts`,
  `roads`, `walk`.
