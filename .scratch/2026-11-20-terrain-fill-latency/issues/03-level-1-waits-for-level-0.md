# 03: A level-1 Cell over level 0's load extent waits for level 0's Chunks, if it is worth it

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

First the count: level-0 children a level-1 job generates though they lie inside level 0's load extent, as a share of level-1 generation in `flight`, `sprint-flight` and `teleport`. At 10% or less, closed `wontfix` with the figure. Otherwise a level-1 Cell all eight of whose level-0 children lie inside level 0's load extent is not dispatched until level 0 holds them; its job then reads them as held Chunks instead of generating them again. Any other level-1 Cell is dispatched as now.

## Acceptance criteria

- [ ] The count in Comments.
- [ ] If built: a streaming test: such a Cell is not dispatched before its children are held, and its Coarse equals one generated alone.
- [ ] If built: the holes instrument reports no more fill or rule holes than 01's figures, the teleport included.
- [ ] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.
