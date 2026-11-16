# 36: Spawn beside a Settlement, with no demo stage at spawn

Status: done

Blocked by: 27

Spec: [Phase 20](../spec.md).

## What to build

Play-testing in a new World: "at spawn point there is this old structure with
lava, water other block. i want you to get rid of this and spawn me near a
village/hamlet".

- Terra's demo stage (a levelled site at the origin with a wall, a lantern
  post, lava, mud and water scenarios and burning posts, built on the first
  tick) goes: Terra's `tick` sends nothing.
- A new World's spawn stands just outside the disc of the Settlement nearest
  the origin, within 512 m, on the side facing the origin or the nearest dry
  side round it. A World with no Settlement that near spawns over the origin,
  as before. Respawn uses the same point.

## Acceptance criteria

- [x] Terra sends no Commands, new or reopened, and leaves the spawn
      neighbourhood's terrain as generated, tested.
- [x] `Worldgen::settlement_spawn_m` puts the spawn just outside the nearest
      Settlement's disc, on dry ground, and `None` with no Settlement near,
      tested.
- [x] The Server's spawn takes it, and keeps the origin otherwise.

## Comments

Where it lives: `Worldgen::settlement_spawn_m` and `SPAWN_OUT_M` (6 m past the
disc, inside the 16 m margin) in `worldgen/settlements.rs`; `spawn_eye` and
`SPAWN_SEARCH_M` (512 m) in `server/mod.rs`. Terra's `State`, `Stage`, site,
build, scenario and batching helpers are gone.

Tests: `the_spawn_stands_just_outside_the_settlement_nearest_the_origin`
(core); `terra_contract_content_loads_and_terra_builds_nothing_at_spawn` keeps
the content half of the old stage test and now checks nothing is sent and the
spawn's Chunks stay as generated; `real_terra_builds_nothing_new_or_reopened`.
The Server's spawn tests still pass: their Worlds have no Settlements.

Coverage lost with the stage: a real guest's Commands batched within the
host's caps over several calls, and Mod-sent ignition. Source placement and
removal through a Mod stay covered by
`a_0_2_mod_places_and_removes_sources_through_the_edit_path`.

The 48 m keep-out at the origin stays, for a Game that builds there.
