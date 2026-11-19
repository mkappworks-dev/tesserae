# 08: Test fixtures declare every content kind

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 05.

## What to build

A content set for the digest tests, appended to the shared fixtures so no
existing id moves: a Biome that declares a Surface feature, a Column Plant that
stands in water, a crop field that reaches toward water, a Structure Template,
and a Settlement with a near-spawn entry, generated under version 3 with ground
and Biomes.

## Acceptance criteria

- [x] A test per kind shows generation places it: the Surface feature, the
      Column Plant in water, the crop field, the Structure and the Settlement.
- [x] Existing fixture ids, and the tests and Server fixtures that assert them,
      are unchanged.
- [x] No generation code is touched.

## Comments

- `testgen::content()` is the set: `params` (version 3, `world_params()`'s ground, sea and two Biomes), `plants`, `templates` and `settlements`, and `Content::worldgen(seed, size)` builds the `Worldgen` with all three. Both Biomes declare a reed (a Column Plant standing in the fixture water), a crop field within 16 m of water with lanes and a water ditch, a cairn Structure and a hamlet Settlement row marked `near_spawn`; `warm` alone also declares a flower Surface feature. `world_params()` itself is unchanged, so no recorded digest moves.
- Its nine Materials, `fixtures().content`, register after water, so every earlier fixture id keeps its number; a testgen test asserts the order and that the set validates. No Shape is added, so the Server's `fixture_tables()` fence keeps its id. The plant Materials are solid Cutout, since `validate_ground` refuses a non-solid surface feature.
- `plants` holds the reed alone: the only Plant whose generated Overlay is not empty.
- Names are left to ticket 09: ticket 07's naming was not on `tech/generator-determinism` when this landed.
- `tests/content_fixtures.rs` shows each kind standing at all four digest cases (both seeds, 1 m and 0.5 m), about 25 s in a debug build. The origin is open sea for kilometres at both seeds, so the tests look round the Generator's spawn point, which is inside the hamlet at every case, and the reed test looks round the deep water (2 m or more) nearest to it, since a reed needs more than a Voxel of water over it.
- `cargo test -p tesserae --features tesserae/dev` fails only ticket 20's two tests, with the same assertions as before.

