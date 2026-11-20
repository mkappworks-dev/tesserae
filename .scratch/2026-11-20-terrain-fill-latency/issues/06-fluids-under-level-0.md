# 06: Fluids stop regenerating level 1 under level 0, if it is worth it

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

First a count: level-1 regenerations and meshes per second in `flight`, by cause (a fluid landing, a neighbour arriving, a first generation). If fluid landings are 10% of level-1 jobs or less, closed `wontfix` with the figure. Otherwise a level-1 Chunk whose eight children level 0 draws is not regenerated or meshed for a fluid change until level 0 stops drawing one of them; ADR-0019's rule covers the handover.

## Acceptance criteria

- [ ] The count in Comments.
- [ ] If built: a streaming test that a fluid change under level 0 queues no level-1 work, and that the level-1 Chunk matches level 0 once one child leaves level 0's ring.
- [ ] If built: the holes instrument reports no more holes than 01's figures.
