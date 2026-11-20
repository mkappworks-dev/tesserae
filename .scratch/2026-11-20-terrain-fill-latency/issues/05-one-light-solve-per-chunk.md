# 05: A light solve waits for the neighbours it reads, if relight is still worth it

Status: ready-for-agent

Blocked by: 02, 03, 04

Spec: [Terrain fill latency](../spec.md).

## What to build

First the count and the gate: level-0 light solves per Chunk by cause (a neighbour arriving, a fluid landing, an Edit), and level 0's relight as a share of worker CPU in `sprint-flight`, after the generation tickets that land before it. At 10% or less, closed `wontfix` with the figure. Otherwise a solve is not dispatched until its four sideways neighbours and the one above are held or outside the load extent, and solves are ordered top-down within distance bands rather than across the whole load extent.

## Acceptance criteria

- [x] The count by cause and the gate's figure in Comments.
- [ ] If built: a streaming test that a solve is not dispatched while a sideways neighbour inside the load extent is missing, and that the settled light equals today's order's.
- [ ] If built: light solves per level-0 Chunk, before and after.
- [ ] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.

## Comments

### The count and the gate, 2026-11-20

| Fixture | Level-0 Chunks generated | First solves | Re-solves for a neighbour | For a fluid | For an Edit |
| --- | --- | --- | --- | --- | --- |
| `flight` | 660 | 660 | 4467 | 0 | 0 |
| `sprint-flight` | 3300 | 3300 | 21189 | 0 | 1 |
| `teleport` | 1639 | 1639 | 5299 | 0 | 0 |

Every extra solve is a fill's: a neighbour's face plane moving, 3.2 to 6.8 times per Chunk. Level 0's relight is 449 ms/s of 1665 in `sprint-flight` at fa6fd574 (27%), over the 10% gate: built.

One `scripts/profile-frames --fixtures flight sprint-flight teleport --repeats 3` series of dec13212 (the counters on fded14c8's successors: the sister spec's 01 to 05 and this spec's 01, 02, 04 and 07), 11:51 to 12:01, `uptime` load 9.86 falling to 5.52; medians of the three logging-on runs per fixture, counted from the World's opening.
