# Default Worlds have lakes

Status: ready-for-agent
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
