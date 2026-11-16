# 04: Oceans, Sea level and the Water surface function (17a)

Status: ready-for-agent

Blocked by: 03

Spec: [Phase 17](../spec.md).

## What to build

Shelf, slope, deep floor, trenches, coasts with beaches, cliffs and stony shores, archipelagos. `water_surface_m(x_m, z_m)` answers the sea at Sea level for now; Settlements, crossings, the Waterline, LOD level 1, spawn and the Map ask it instead of `water.level_m` under version 3.

## Acceptance criteria

- [ ] No consumer reads `water.level_m` under version 3.
- [ ] A World with no Game fluid has the basins, dry.
- [ ] Coarse levels draw the same coast as level 0 to within a coarse Voxel.
