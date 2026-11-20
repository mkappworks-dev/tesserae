# 01: How far level 0 lags, measured

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Terrain fill latency](../spec.md).

## What to build

A profiling instrument beside the holes count, off unless asked for: per frame, the distance from the Eye to the nearest Cell in the frustum that level 0's ring holds and level 0 does not draw, whatever coarser level covers it; per level-0 Chunk, the time from entering the mesh extent to its mesh being drawn, as percentiles per 32 m band. The Server's streaming stats record when a Chunk enters the mesh extent, the Client records when its mesh is drawn from what it was sent, and the profiler joins the two; no Client system reads or mutates World state. `scripts/profile-frames` summarises both per fixture, and the starting figures are recorded with it, with job counts per Chunk per level and stage.

## Acceptance criteria

- [x] The instrument is in the CSV and the runner's summary, and costs nothing when off.
- [x] A Client test: a level-0 Cell covered only by a coarser level counts as missing; one level 0 draws does not.
- [ ] The starting figures of `flight`, `sprint-flight` and `teleport`, three runs each, with `uptime`, in Comments.

## Comments

- Built: `--instruments lag` sets `TESSERAE_PROFILE_LAG`. `frames.csv` gains `level0_missing_m` (`client/lag.rs`, `inf` when nothing is missing); `frames.fill.csv` has each level-0 Chunk's entry into the mesh extent (recorded in the Server's streaming stats) joined with the Chunks the Client starts and stops drawing (`profiling/fill.rs`). `streaming.json` reports `level0_missing_m` (min, p1, p5, median, nearer being worse), `fill` per 32 m band of the distance at entry, and `relight_per_chunk` and `mesh_per_chunk` per level, over the fixture phase and per move. Off, the Client installs nothing and the Server checks one flag a tick.
- Tests: `a_level_zero_cell_covered_only_by_a_coarser_level_is_missing` and `a_cell_level_zero_draws_is_not_missing` (Client), `a_chunk_enters_the_mesh_extent_once_as_the_anchor_moves_to_it` (Server), `a_chunk_is_drawn_after_entering_when_its_mesh_first_lands` and `a_chunk_still_drawn_as_it_enters_is_drawn_at_once` (the join). The Client and join tests need `--features profiling`, as the holes tests do. The runner's summary was checked on a synthetic run folder, not on a real run.
