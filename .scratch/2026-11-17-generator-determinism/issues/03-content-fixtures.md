# 03: Test fixtures declare every content kind

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

A content set for the digest tests, built beside the shared fixtures and
appended to them so no existing id moves: a Biome that declares a Surface
feature, a Column Plant that stands in water, a crop field that reaches toward
water, a Structure Template, and a Settlement with a near-spawn entry. Versions
2 and 3 generate it with ground and Biomes.

## Acceptance criteria

- [ ] A test per kind shows generation places it: the Surface feature, the
      Column Plant in water, the crop field, the Structure and the Settlement,
      under versions 2 and 3.
- [ ] Existing fixture ids, and the tests and Server fixtures that assert them,
      are unchanged.
- [ ] No generation code is touched.
