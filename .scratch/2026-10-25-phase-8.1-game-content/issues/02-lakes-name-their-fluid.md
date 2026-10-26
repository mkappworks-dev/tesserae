# 02: Generated lakes are made of a named fluid

Status: done
Blocked by: 01

Spec: [decisions 4 and 5](../spec.md).

## What to build

The engine's World parameters for standing fluid carry the lake's fluid
Material beside its level. Generation fills open air below the level with
packed cubes of that Material. Everything derived from the lake reads it:
- slope smoothing yields to it;
- the level-1 waterline counts only packed lake-fluid cubes below the level as
  generated;
- coarse under-water light dims by its own dimming;
- the Server's cut of a coarse water surface to level 0's height
  (`server/lod.rs`) applies to the lake fluid's top faces;
- the spawn point clears the level.

The engine's default World parameters have no lakes. Until tickets 03 and 04
let a Game name a fluid, the Server keeps today's behaviour by naming the
built-in water as the lake fluid when no Game does. That bridge is removed in
ticket 09.

## Acceptance criteria

- [x] A World whose parameters name a fluid generates lakes of that fluid. A
      lava lake works the same way, and a lake of any fluid Occupants reads as a
      source.
- [x] The engine's default World parameters have no lakes, and generation with
      them places no fluid.
- [x] Validation refuses a lake Material that is unknown, solid or has no fluid
      row, naming the field.
- [x] Smoothing never replaces lake fluid of any Material. The existing
      underwater-step tests pass with a non-water lake fluid too.
- [x] Level 1 treats only the lake's own Material below the level as generated,
      and any other fluid there as dynamic. Coarse under-water light matches
      level 0 for a lake fluid with a dimming other than water's.
- [x] The spawn point is above a lake of any fluid.
- [x] `TESSERAE_WATER_LEVEL_M` moves the level when a lake fluid is named, and
      warns and changes nothing when none is.
- [x] `cargo dev` with no Game still shows today's lakes, through the temporary
      built-in-water bridge.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

- `WaterParams` gains `material`. `WorldgenParams::default()` has no lake;
  `with_default_lake(material)` puts one at the old default level.
  `validate_lake(&tables)` refuses an unknown, solid or rowless Material as
  `water.material`. The Server calls it on the Game's World parameters once
  `resolve` has registered every fluid row.
- `Worldgen::water()` replaces `water_level_m()` and returns the clamped lake.
  `Waterline` and `UnderWater` carry the Material, so level 1 treats only
  packed cubes of the lake's own fluid as generated. Coarse light dims by that
  fluid's dimming, and the Server's coarse surface cut (`level_the_water`)
  moves only that fluid's top faces. Smoothing already yielded to any non-air
  cell.
- The bridge: `server::bridged_default` gives a World whose parameters come
  from no Game lakes of the built-in water, and a 0.2 standing-water record
  converts to the built-in water (`tesserae-mods` `convert.rs`). Both are
  marked for ticket 09 and ticket 03 respectively. The demo already declares
  `water: None`, so it is unaffected.
- `TESSERAE_WATER_LEVEL_M` keeps the lake's Material and moves its level; with
  no lake it logs a warning and leaves the World dry. `=off` is unchanged.
- The ticket 01 sites waiting on this moved to the fixture water: worldgen's
  lake tests (now also run with a hot, opaque, non-waterlogging lava), the LOD
  generators and waterline tests (both directions: a lake of either fluid,
  the other dynamic below it), coarse light against level 0 for water and a
  translucent fluid with dimming 4, the shaft-under-a-lake flow test, the
  Server's `coarse_water` and `held.rs` lakes, and the `shore` and `rescans`
  benches, which now generate fixture-water lakes with the same rows.
- "`cargo dev` with no Game still shows today's lakes" is checked by
  `a_world_with_no_game_keeps_its_lakes_of_the_built_in_water`, not by eye.
- `./scripts/cargo-ci` passes.
