# 03: A level-1 Cell over level 0's load extent waits for level 0's Chunks, if it is worth it

Status: wontfix

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

First the count: level-0 children a level-1 job generates though they lie inside level 0's load extent, as a share of level-1 generation in `flight`, `sprint-flight` and `teleport`. At 10% or less, closed `wontfix` with the figure. Otherwise a level-1 Cell all eight of whose level-0 children lie inside level 0's load extent is not dispatched until level 0 holds them; its job then reads them as held Chunks instead of generating them again. Any other level-1 Cell is dispatched as now.

## Acceptance criteria

- [x] The count in Comments.
- [ ] ~~If built: a streaming test: such a Cell is not dispatched before its children are held, and its Coarse equals one generated alone.~~
- [ ] ~~If built: the holes instrument reports no more fill or rule holes than 01's figures, the teleport included.~~
- [ ] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.

## Comments

### The count, 2026-11-20

| Fixture | Level-0 Chunks level-1 jobs generated | Of them inside level 0's load extent | Level-1 jobs with one |
| --- | --- | --- | --- |
| `flight` | 3190 | 2 | 1 |
| `sprint-flight` | 14195 | 13 | 6 |
| `teleport` | 11755 | 0 | 0 |

Next to nothing, under the 10% gate: closed. A level-1 Cell whose children level 0 loads is dispatched after level 0 holds them already, so level 1 does not generate them twice.

One `scripts/profile-frames --fixtures flight sprint-flight teleport --repeats 3` series of dec13212 (the counters on fded14c8's successors: the sister spec's 01 to 05 and this spec's 01, 02, 04 and 07), 11:51 to 12:01, `uptime` load 9.86 falling to 5.52; medians of the three logging-on runs per fixture, counted from the World's opening.
