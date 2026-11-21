# 04: Structure Impostors

Status: ready-for-agent
Blocked by: 03

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md),
[ADR-0029](../../../docs/adr/0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md).

## What to build

Buildings and villages show on the horizon. Every Structure and Settlement Plot
that level 0 stands is sampled at each coarse level's Voxel size, any solid
beating air, from 03's placement generator. Settlement paths are ground
Material. Over saved changes, the changed Voxels are used, as in 03.

## Acceptance criteria

- [ ] The placement generator's Structures and Plots equal level 0's for the
      same region.
- [ ] A Structure's Impostor occupies the coarse Voxels its level-0 Voxels
      reach, at every coarse level.
- [ ] A Creative placement or removal, once saved and evicted, shows at the
      coarse levels as it does today.
- [ ] Coarse-Chunk time and memory against 03's, in Comments.
- [ ] Captures of a Settlement at 0.5, 1 and 2 km, shown to you.
