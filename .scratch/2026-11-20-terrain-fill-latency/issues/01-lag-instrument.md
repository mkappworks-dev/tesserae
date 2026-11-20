# 01: How far level 0 lags, measured

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Terrain fill latency](../spec.md).

## What to build

A profiling instrument beside the holes count, off unless asked for: per frame, the distance from the Eye to the nearest Cell in the frustum that level 0's ring holds and level 0 does not draw, whatever coarser level covers it; per level-0 Chunk, the time from entering the mesh extent to its mesh being drawn, as percentiles per 32 m band. The Server's streaming stats record when a Chunk enters the mesh extent, the Client records when its mesh is drawn from what it was sent, and the profiler joins the two; no Client system reads or mutates World state. `scripts/profile-frames` summarises both per fixture, and the starting figures are recorded with it, with job counts per Chunk per level and stage.

## Acceptance criteria

- [ ] The instrument is in the CSV and the runner's summary, and costs nothing when off.
- [ ] A Client test: a level-0 Cell covered only by a coarser level counts as missing; one level 0 draws does not.
- [ ] The starting figures of `flight`, `sprint-flight` and `teleport`, three runs each, with `uptime`, in Comments.
