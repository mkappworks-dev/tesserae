# 01: Version 3 fixture parameters for tests

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

The shared test fixtures offer version 3 parameters (ground, Biomes, landforms,
water) beside today's version 2 ones, so tests can move onto version 3 one batch
at a time while the rest still build. Nothing else changes.

## Acceptance criteria

- [ ] A test can ask the fixtures for version 3 parameters and generate Chunks
      with them.
- [ ] Every existing test and bench passes unchanged.
- [ ] No generation code is touched.
