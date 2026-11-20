# 05: A light solve waits for the neighbours it reads, if relight is still worth it

Status: ready-for-agent

Blocked by: 02, 03, 04

Spec: [Terrain fill latency](../spec.md).

## What to build

First the count and the gate: level-0 light solves per Chunk by cause (a neighbour arriving, a fluid landing, an Edit), and level 0's relight as a share of worker CPU in `sprint-flight`, after the generation tickets that land before it. At 10% or less, closed `wontfix` with the figure. Otherwise a solve is not dispatched until its four sideways neighbours and the one above are held or outside the load extent, and solves are ordered top-down within distance bands rather than across the whole load extent.

## Acceptance criteria

- [x] The count by cause and the gate's figure in Comments.
- [x] If built: a streaming test that a solve is not dispatched while a sideways neighbour inside the load extent is missing, and that the settled light equals today's order's.
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

### Built, 2026-11-20

`dispatch_relight` skips a Chunk until its four sideways neighbours and the one above are held or lie outside the load extent (`neighbours_held`); the solve order is `Order::top_down_within_bands`, top-down within bands two Cells of priority wide, a column kept whole in the band of its best Chunk. Landing, the Edit path and the spoilt-solve fallback are unchanged.

- `a_solve_waits_for_the_neighbours_it_reads_inside_the_load_extent`: with a Chunk removed and its generation held, neither the Chunk beside it nor the one below is dispatched, a Chunk at the load extent's edge is, and the settled light equals the synchronous sweep's fixed point (`fixed_point`, the light today's order reaches).
- `columns_are_solved_top_down_band_by_band` (`priority`).
- `a_fresh_region_is_lit_without_solving_chunks_again_for_light_from_above`, bound 3.75 → 2.2. Five interleaved runs each, at load 19: solved as soon as it lands 2.23 to 2.43 per generated Chunk; with the gate 2.00 to 2.05 in one band, 2.13 to 2.21 in bands of one Cell, 1.99 to 2.07 in bands of two (the default), 2.04 to 2.08 in bands of four. Also waiting for the solve above to land, not just the Chunk: 2.12 to 2.17, so not built.
