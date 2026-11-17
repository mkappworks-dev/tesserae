# 05: The coarse levels share level 0's terrain and columns

Status: ready-for-agent

Blocked by: 02

Spec: [Worldgen performance](../spec.md).

## What to build

A generator can be made to share another's work when its seed and every terrain parameter match: it always adopts the other's terrain, so one drainage solve serves every level, and adopts the column cache only at the same Voxel size. The Server makes its coarse levels' sources share level 0's generator; the level-1 source, level 0 without surface features, shares the column cache too. The direct column generators of the farthest levels stay as they are.

## Acceptance criteria

- [ ] A shared source generates the same Chunks as one made alone, at every level, warm or cold.
- [ ] Sharing a generator whose seed or terrain differs is refused.
- [ ] Version 3 Chunks match the ticket 01 fingerprint; versions 1 and 2 keep their recorded digests.
- [ ] Bench figures and the coarse levels' worker time before and after in Comments.
