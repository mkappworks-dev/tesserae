# 09: Digests cover every content kind and spawn

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 06, 07, 08.

## What to build

Digest cases over 08's content, recorded on macOS against today's code. Each
`(seed, Voxel size)` case adds a box on the spawn point sized to reach the
Settlement's disc and a box on a Structure, found by sweeping generated Chunks
for the Template's Material. One case's origin stands in the sea, so the dry
land search runs. Each Voxel's generated Overlay, derived through the Plant
table, is hashed by fluid name, level and source flag. Each case records its
spawn position from 06's query.

## Acceptance criteria

- [x] The other recorded-digest tests (`coarse_generation`, `surface_features`)
      hash by name through ticket 07's mapping too, re-recorded on macOS.
- [x] Each box asserts what it holds before its digest is compared: a Settlement
      building, a Structure, a Plant, water.
- [x] Spawn positions are recorded per case and compared bit for bit.
- [x] Some case reaches each site the correction touches: Settlement founding,
      the spawn ring's angle step, roads, building orientation, the level-disc
      test, crop field reach near water, the dry land search, Surface feature
      chance, tree crowding, and the landform and wetness `powi` (the existing
      boxes may be cited). Shown once with a temporary counter, not committed,
      and noted in a comment here.
- [x] The digest tests take under a minute in a release build; the time is noted
      in a comment here.

## Comments

- Recorded on this Mac (macOS, arm64) against today's code: no generation code changed. The new cases are in `tests/generator_version.rs`, at both content seeds (`0x7e55_e2ae_0000_0001`, `0xdead_beef_cafe_f00d`) and both Voxel sizes, so four cases of five recordings each: a box on the spawn point reaching the Settlement's disc (asserts a building and a road), a box on the Structure nearest spawn found by sweeping Chunks outward for `t:stone` (asserts the Structure), a box on the reed nearest spawn's deep water found the same way (asserts the Plant standing in source water), a box on the forest's spawn (asserts trunk and canopy), and the spawn positions, compared bit for bit.
- The forest is test-local: `world_params()`' ground and sea under one Biome of trees 6 m apart, and no Settlement. The fixtures had no tree, so nothing reached tree crowding; and with no Settlement the spawn query falls to the dry land search, since the origin is sea at every case (asserted). Its trunk is an unregistered id the test's mapping names `t:wood`, as `surface_features` does for its own features.
- `testgen::content()`'s hamlet gains an outskirts group of huts: building orientation's angle is only in the outskirts. `content_fixtures` still passes with it.
- The Overlay is hashed per Voxel after its Chunk: a 0 where no fluid, else a 1, the fluid's name, its source flag and each void's surface, reach and descent. The name encoding (a little-endian `u32` length, then UTF-8) is written once, in `tests/digest/mod.rs`, which `coarse_generation` and `surface_features` now share; both were re-recorded on this Mac. `generator_version`'s twelve terrain digests did not move.
- Reach, shown once with a temporary counter at each site (not committed), counting calls per test run alone: Settlement founding (`near_spawn_draw`, 232), the spawn ring's angle step (4), roads (`atan2` 20, `walk` 4200), building orientation (`lay_outskirts`, 804), the level-disc test (2642) and crop field reach near water (125 810) by the content boxes; the dry land search by the forest and spawn cases (4 each); Surface feature chance by every case (`SurfaceScatter::new` everywhere, `chance` in the content and forest cases); tree crowding by the forest (1195); the landform `powi` (outcrops, islands, volcanoes) by every box, the existing origin box included; wetness's `powi` by the high-ground box (900) and the content boxes (136), not the origin box. Every site is reached.
- Time in a release build, at a 1-minute load average of 5.4 to 5.9 on 10 cores: `generator_version` 1.6 to 1.8 s (10 tests), `coarse_generation` 4.0 to 4.4 s, `surface_features` 4.5 to 5.1 s; about 11 s for the three.
