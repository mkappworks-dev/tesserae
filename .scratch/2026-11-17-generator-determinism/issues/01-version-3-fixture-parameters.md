# 01: Version 3 fixture parameters for tests

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

The shared test fixtures offer version 3 parameters (ground, Biomes, landforms,
water) beside today's version 2 ones, so tests can move onto version 3 one batch
at a time while the rest still build. Nothing else changes.

## Acceptance criteria

- [x] A test can ask the fixtures for version 3 parameters and generate Chunks
      with them.
- [x] Every existing test and bench passes unchanged.
- [x] No generation code is touched.

## Comments

- `testgen::world_params_v3()` is version 3 on the fixture ground: default
  landforms, fixture water at a 24 m Sea level, and Biomes `cold` and `warm`. It
  builds on `world_params()`, so the caves, smoothing and climate match the
  version 2 fixture's.
- Tested by `version_3_params_generate_ground_water_and_biomes`: it validates
  against the fixture tables, and the sampled Chunks hold the fixture surface,
  water and both Biomes.
