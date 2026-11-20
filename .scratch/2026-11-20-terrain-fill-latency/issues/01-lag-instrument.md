# 01: How far level 0 lags, measured

Status: done

Blocked by: None (can start immediately)

Spec: [Terrain fill latency](../spec.md).

## What to build

A profiling instrument beside the holes count, off unless asked for: per frame, the distance from the Eye to the nearest Cell in the frustum that level 0's ring holds and level 0 does not draw, whatever coarser level covers it; per level-0 Chunk, the time from entering the mesh extent to its mesh being drawn, as percentiles per 32 m band. The Server's streaming stats record when a Chunk enters the mesh extent, the Client records when its mesh is drawn from what it was sent, and the profiler joins the two; no Client system reads or mutates World state. `scripts/profile-frames` summarises both per fixture, and the starting figures are recorded with it, with job counts per Chunk per level and stage.

## Acceptance criteria

- [x] The instrument is in the CSV and the runner's summary, and costs nothing when off.
- [x] A Client test: a level-0 Cell covered only by a coarser level counts as missing; one level 0 draws does not.
- [x] The starting figures of `flight`, `sprint-flight` and `teleport`, three runs each, with `uptime`, in Comments.

## Comments

- Built: `--instruments lag` sets `TESSERAE_PROFILE_LAG`. `frames.csv` gains `level0_missing_m` (`client/lag.rs`, `inf` when nothing is missing); `frames.fill.csv` has each level-0 Chunk's entry into the mesh extent (recorded in the Server's streaming stats) joined with the Chunks the Client starts and stops drawing (`profiling/fill.rs`). `streaming.json` reports `level0_missing_m` (min, p1, p5, median, nearer being worse), `fill` per 32 m band of the distance at entry, and `relight_per_chunk` and `mesh_per_chunk` per level, over the fixture phase and per move. Off, the Client installs nothing and the Server checks one flag a tick.
- Tests: `a_level_zero_cell_covered_only_by_a_coarser_level_is_missing` and `a_cell_level_zero_draws_is_not_missing` (Client), `a_chunk_enters_the_mesh_extent_once_as_the_anchor_moves_to_it` (Server), `a_chunk_is_drawn_after_entering_when_its_mesh_first_lands` and `a_chunk_still_drawn_as_it_enters_is_drawn_at_once` (the join). The Client and join tests need `--features profiling`, as the holes tests do. The runner's summary was checked on a synthetic run folder, not on a real run.

### The starting figures, 2026-11-20

One `scripts/profile-frames --fixtures flight sprint-flight teleport --repeats 3 <out> --instruments lag holes` series of this commit's build (fded14c8, the fded14c8 rows) at 10:21, load 5.8, beside fa6fd574's at 10:31 for the tickets after it. Per run: frames in which level 0 is missing from some Cell in view, the nearest such Cell, frames with a fill hole, and the time from a Chunk entering the mesh extent to being drawn:

| Fixture | Build | Frames missing level 0 | Nearest missing, min | Holes (fill frames) | Draw time p50 / p95 / max, 128–160 m | Settle |
| --- | --- | --- | --- | --- | --- | --- |
| `flight` | fded14c8 | 121 / 7652 | 127 m | 0/0/0 | 0.05/0.08/0.08 · 0.04/0.07/0.07 · 0.04/0.08/0.08 | – |
| `flight` | fa6fd574 | 76 / 6871 | 128 m | 0/0/0 | 0.03/0.04/0.05 · 0.03/0.05/0.05 · 0.03/0.05/0.05 | – |
| `sprint-flight` | fded14c8 | 653 / 6872 | 124 m | 8/8/8 | 0.03/0.07/0.10 · 0.04/0.08/0.08 · 0.04/0.07/0.09 | – |
| `sprint-flight` | fa6fd574 | 382 / 6311 | 124 m | 7/8/11 | 0.02/0.05/0.08 · 0.02/0.05/0.06 · 0.02/0.05/0.06 | – |
| `teleport` | fded14c8 | 1365 / 7256 | 0 m | 118/156/74 | – | 2.08 · 2.14 · 2.13 |
| `teleport` | fa6fd574 | 1282 / 6876 | 0 m | 98/96/112 | – | 2.02 · 2.13 · 2.17 |

Level 0 keeps up in both flights: it is missing only at the edge of its 160 m ring, never nearer than 124 m, even at 64 m/s, and a Chunk is drawn within 0.1 s of entering the mesh extent. The spec's flight targets (never missing within 96 m at 12 m/s; a median of at least 64 m at 64 m/s) already hold. The teleport, which refills the view as a spawn does, is where level 0 lags: missing at the Eye for about 2 s, settling in 2.1 s against the target of under 1 s. Worker CPU and jobs per Chunk for the same builds are in [the sister spec's ticket 05](../../2026-11-20-generation-cost/issues/05-small-caches.md#the-flight-after-05-2026-11-20).
