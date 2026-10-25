# 02: Generated lakes are made of a named fluid

Status: ready-for-agent
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
- the spawn point clears the level.

The engine's default World parameters have no lakes. Until tickets 03 and 04
let a Game name a fluid, the Server keeps today's behaviour by naming the
built-in water as the lake fluid when no Game does. That bridge is removed in
ticket 09.

## Acceptance criteria

- [ ] A World whose parameters name a fluid generates lakes of that fluid. A
      lava lake works the same way, and a lake of any fluid Occupants reads as a
      source.
- [ ] The engine's default World parameters have no lakes, and generation with
      them places no fluid.
- [ ] Validation refuses a lake Material that is unknown, solid or has no fluid
      row, naming the field.
- [ ] Smoothing never replaces lake fluid of any Material. The existing
      underwater-step tests pass with a non-water lake fluid too.
- [ ] Level 1 treats only the lake's own Material below the level as generated,
      and any other fluid there as dynamic. Coarse under-water light matches
      level 0 for a lake fluid with a dimming other than water's.
- [ ] The spawn point is above a lake of any fluid.
- [ ] `TESSERAE_WATER_LEVEL_M` moves the level when a lake fluid is named, and
      warns and changes nothing when none is.
- [ ] `cargo dev` with no Game still shows today's lakes, through the temporary
      built-in-water bridge.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
