# 05: A light solve waits for the neighbours it reads, if relight is still worth it

Status: done

Blocked by: 02, 03, 04

Spec: [Terrain fill latency](../spec.md).

## What to build

First the count and the gate: level-0 light solves per Chunk by cause (a neighbour arriving, a fluid landing, an Edit), and level 0's relight as a share of worker CPU in `sprint-flight`, after the generation tickets that land before it. At 10% or less, closed `wontfix` with the figure. Otherwise a solve is not dispatched until its four sideways neighbours and the one above are held or outside the load extent, and solves are ordered top-down within distance bands rather than across the whole load extent.

## Acceptance criteria

- [x] The count by cause and the gate's figure in Comments.
- [x] If built: a streaming test that a solve is not dispatched while a sideways neighbour inside the load extent is missing, and that the settled light equals today's order's.
- [x] If built: light solves per level-0 Chunk, before and after.
- [x] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.

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

### Figures, 2026-11-20

| Fixture | Build | L0 solves | L0 relight ms/s | L1 generations | L1 meshes | L1 mesh ms/s | All stages ms/s |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `flight` | 4e78a50f | 4963 | 97.3 | 1010 | 4469 | 309.0 | 1008.5 |
| `flight` | fa6fd574 | 4860 | 102.0 | 1036 | 4602 | 302.6 | 758.2 |
| `flight` | e414dbef | 3722 | 79.2 | 791 | 2849 | 202.2 | 630.8 |
| `sprint-flight` | 4e78a50f | 25019 | 432.3 | 2083 | 3190 | 154.4 | 2405.6 |
| `sprint-flight` | fa6fd574 | 24536 | 449.0 | 2205 | 4064 | 199.2 | 1664.8 |
| `sprint-flight` | e414dbef | 18434 | 357.2 | 2052 | 2919 | 149.4 | 1606.8 |
| `teleport` | 4e78a50f | 7065 | 93.9 | 1978 | 2600 | 135.5 | 1893.9 |
| `teleport` | fa6fd574 | 6924 | 98.0 | 2026 | 2903 | 146.2 | 1379.9 |
| `teleport` | e414dbef | 6570 | 114.5 | 1841 | 1738 | 96.2 | 1473.0 |

Against fa6fd574 (before this ticket and 06), level-0 light solves fall 23% at 12 m/s and 25% at 64 m/s (to 5.6 a Chunk), relight CPU 22% and 20%. After a teleport the solves fall 5%, but the near ground is lit first: level 0 is whole within 32 m of the Eye in under a second, about twice as soon:

| Level 0 complete within, after a teleport | fded14c8 | fa6fd574 | e414dbef |
| --- | --- | --- | --- |
| 32 m | 1.77 to 1.84 s | 1.63 to 1.88 s | 0.91 to 0.97 s |
| 64 m | 1.85 to 1.89 s | 1.82 to 1.89 s | 1.06 to 1.12 s |
| 96 m | 1.93 to 2.01 s | 1.88 to 2.05 s | 1.07 to 1.18 s |

The full settle, which waits for the ring's far edge too, reads 2.24 s (2.16 to 2.41) against 2.72 s at 4e78a50f in the same rounds and 2.05 s at fa6fd574: lighting near Chunks first leaves the edge later.

How they were taken: e414dbef (`tesserae-end`, both specs' tickets but this spec's 08) against 4e78a50f, three interleaved rounds of `scripts/profile-frames --fixtures flight sprint-flight teleport --repeats 3` (end, start; start, end; end, start) from 12:40 to 13:41, then one `--instruments lag holes` series of e414dbef to 13:51; nothing else built or ran a `tesserae` process. `uptime` load 22.2 at the first series' start (a build had just ended), then 5.7 to 7.0. fa6fd574's figures are its two rounds from the sister spec's ticket 05. Medians over logging-on runs. "Level 0 complete within R" is the time from the teleport to the last frame in which `level0_missing_m` is under R, from each lag series' `frames.csv`, one figure per run.
