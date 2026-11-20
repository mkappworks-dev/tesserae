# 03: A level-1 Cell over level 0's load extent waits for level 0's Chunks

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

A level-1 Cell whose level-0 children lie inside level 0's load extent is not dispatched until level 0 holds them; its job then reads them as held Chunks instead of generating them again. A Cell outside level 0's load extent is dispatched as now.

## Acceptance criteria

- [ ] A streaming test: such a Cell is not dispatched before its children are held, and its Coarse equals one generated alone.
- [ ] The holes instrument reports no more fill or rule holes than 01's figures.
- [ ] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.
