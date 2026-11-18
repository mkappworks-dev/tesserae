# 08: Test fixtures declare every content kind

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: 05.

## What to build

A content set for the digest tests, appended to the shared fixtures so no
existing id moves: a Biome that declares a Surface feature, a Column Plant that
stands in water, a crop field that reaches toward water, a Structure Template,
and a Settlement with a near-spawn entry, generated under version 3 with ground
and Biomes.

## Acceptance criteria

- [ ] A test per kind shows generation places it: the Surface feature, the
      Column Plant in water, the crop field, the Structure and the Settlement.
- [ ] Existing fixture ids, and the tests and Server fixtures that assert them,
      are unchanged.
- [ ] No generation code is touched.
