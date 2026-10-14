# Default Worlds have lakes

Status: done
Blocked by: 06

Worldgen places water only under `dev`'s `TESSERAE_WATER_LEVEL_M`. Decided: a
global water level on by default, at about the 25th percentile of the ground,
in metres.

## What to build

- `WorldgenParams::default()` sets `water` to a level derived from
  `sea_level_m` and `relief_m` (so a Mod that changes either moves the water
  with them). `TESSERAE_WATER_LEVEL_M` still overrides it, and `=off` (or a
  value below the band) turns it off for an A/B.
- Spawn stays on dry ground: if the spawn column is under water, the Eye starts
  above the water's surface.
- Benches that measure terrain keep measuring the dry World they always have,
  by setting `water: None`, and the new default gets its own bench cases.

## Acceptance criteria

- A default World has water in its valleys at level 0 and at every coarse level
  (capture).
- The existing worldgen, meshing and light benches are unchanged against `main`
  (they run dry); the wet cases are reported in absolute terms.
- The streaming budget at level 0 and the per-level fill are re-measured with
  water on, and DESIGN.md's tables say which they are.
- `./scripts/cargo-ci` passes.

## Comments

Built, with one finding the ticket did not foresee.

- **The default.** `WaterParams { level_m }` is set in
  `WorldgenParams::default()` to `sea_level_m + relief_m * 5/12` (44 m), about
  the 25th percentile of the default ground. It stays absolute metres; a Mod
  that changes the relief sets the water too, which the field's comment says.
  `TESSERAE_WATER_LEVEL_M` overrides it under `dev`, and `=off` removes it.
- **Smoothing yields to water.** Slope smoothing put its stair or slab in the
  cell above a one-Voxel step, which on a lake bed is water: the Voxel would
  have replaced the water and left an air pocket in its missing half. A step
  under water now stays a step (`smoothing_never_replaces_water`). Six core
  tests had assumed a dry World; the ones about terrain, caves and smoothing
  now say `water: None`, and the water test compares unsmoothed Worlds.
- **Spawn.** The Eye starts above the water if the ground at the origin is
  under it (`spawn_eye`, tested with the water raised over the spawn).
- **Benches.** Every existing case pins the dry World (`dry()` in `meshing.rs`
  and `lod.rs`), so it stays comparable with `main`; two new cases measure a
  shore Chunk as the default World makes it (`worldgen/shore`,
  `mesh_chunk/shore`).
- **Capture.** A default World shows lakes at level 0 and through the coarse
  levels (tinted capture, 30 s settle).
- **Streaming, five interleaved runs each, load 2.5 to 6.3:** against
  `origin/main` (built in its own worktree), level 0 is shown at 1.76 s
  median with lakes and 1.65 s on `main`, level 5 fills at 11.24 s and 11.19 s,
  the worst tick is 0.9 and 0.8 ms (one lakes run at 9.0 ms), and vertex and
  index memory is 134.5 MB against 135.6 MB: flooded valleys hide more ground
  than water adds. The load volume's fill was 0.4 s later on the branch with or
  without water, and four more interleaved runs with `TESSERAE_TRANSLUCENT_ROW=0`
  matched `main` (1.73 s against 1.72 s): it is the `dev` gallery's new row
  being built through the edit path, not the engine.
- **Core benches against `origin/main`**, interleaved, load 2 to 9. Worldgen is
  unchanged (`straddling_unsmoothed` 106.5 against 105.7 us, `ramp_densest`
  176.3 against 174.7, `straddling_caved` 258.0 against 259.6). `greedy_mesh`
  read 4 to 6% slower and is not: with `CARGO_PROFILE_BENCH_CODEGEN_UNITS=1` on
  both sides `greedy_mesh/terrain` is 327.7 against 335.8 us, the code-layout
  effect the LOD step met. `mesh_chunk/straddling` stays 4 to 5% slower with
  one codegen unit (417.8 against 397.8 us) and is ticket 10's, about 4%, examined in ticket 16: the mesher's
  changes on this branch are a third layer and two more bits in the pass-2
  merge key, and `main` itself spread from 397 to 565 us over the same runs. A
  first reading of `mesh_chunk/ramp_densest` at +33% was `main`'s outlier: a
  bisect re-measured `main` at 444 to 547 us, level with every branch commit.
  New, no baseline: `worldgen/shore` 138 to 178 us.

