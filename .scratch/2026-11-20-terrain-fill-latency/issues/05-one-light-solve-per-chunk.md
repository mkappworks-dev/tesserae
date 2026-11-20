# 05: A light solve waits for the neighbours it reads, if relight is still worth it

Status: ready-for-agent

Blocked by: 02, 03, 04

Spec: [Terrain fill latency](../spec.md).

## What to build

First the gate: level 0's relight as a share of worker CPU in `sprint-flight`, after the generation tickets that land before it. At 10% or less, closed `wontfix` with the figure. Otherwise a solve is not dispatched until its four sideways neighbours and the one above are held or outside the load extent, and solves are ordered top-down within distance bands rather than across the whole load extent.

## Acceptance criteria

- [ ] The gate's figure in Comments.
- [ ] If built: a streaming test that a solve is not dispatched while a sideways neighbour inside the load extent is missing, and that the settled light equals today's order's.
- [ ] If built: light solves per level-0 Chunk, before and after.
- [ ] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.
