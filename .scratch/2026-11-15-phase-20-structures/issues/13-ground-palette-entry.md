# 13: A Template takes the local ground

Status: ready-for-agent

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

A palette entry may be `ground`. Where a Structure stands, it is the Biome's
surface Material at and above the Ground layer and its soil below, so one
Template fits grass, sand or snow.

## Acceptance criteria

- [ ] The same Template stands on grass in a meadow Biome and on sand in a
      desert Biome, by test.
- [ ] A Structure across a Biome border takes each column's own Biome.
- [ ] `ground` passes the Obtainable check by definition.
