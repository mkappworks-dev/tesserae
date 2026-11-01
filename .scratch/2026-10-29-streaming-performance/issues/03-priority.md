# 03: Dispatch by view and direction of travel

Status: ready-for-agent
Blocked by: 01

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-priority`.

## What to build

On the Server, from its own Player state (ADR-0008); the Client sends nothing
new.

- One score for every stage and level: metres from the Eye predicted 1 s ahead
  along its velocity (capped at the view distance) to the Chunk's nearest
  point, halved inside a fixed 60 degree half-angle cone along the look, with
  its apex 6 m behind the Eye. Chunks within 2 Cells always count as inside the
  cone, so a turn finds them ready. An age credit means nothing starves. The
  constants are tuned here against the fixtures; Distant Horizons'
  `distance / detailLevel` is a candidate shape (see the spec's Prior art).
- Two passes: holes first, meaning regions with nothing sent and still held at
  any level, coarsest missing level first; then refinements by score. This
  replaces the level-by-level walk over nearest-first offsets in `lod.rs`'s
  `dispatch` and the `(level, distance)` sorts in its `land` and `land_meshes`.
  Level 0's nearest-first walks in `streaming.rs` take the same score.
- Relight orders columns by score and stays top-down within a column.
- Re-score only on an anchor Cell change, a look change over 15 degrees or a
  speed change over 25%.
- Caps unchanged: 64/128/128 for level 0 and 16/32 coarse.

## Acceptance criteria

- [ ] Re-scoring costs at most 0.2 ms of tick, from the profile.
- [ ] Fewer fill-hole frames in `sprint-flight` and `dive` than 01's baseline,
      and time from `teleport` to `streaming_settled` does not regress.
- [ ] Relight solves per Chunk do not increase.
- [ ] No Chunk waits unboundedly: a starvation test with a fixed look.
- [ ] Frame p50 and p99 do not regress.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.

## Comments

### 2026-10-31: relight goes layer by layer

Branch `tech/streaming-priority-relight`. The ACs on teleport settle, frame
p50/p99 and re-score cost wait for the final combined series; Status unchanged.

- Cause: the column order. Relight now dispatches and lands highest layer
  first, each layer by score (`Order::layers_top_down`, replacing
  `columns_top_down`). This departs from "Relight orders columns by score and
  stays top-down within a column" above.
- Test, written first:
  `a_fresh_region_is_lit_without_solving_chunks_again_for_light_from_above`
  fills a fresh region (96 m view, 64 m vertical, default budget, 3-thread
  pool) and counts relight solves per generated Chunk. Column order 4.25
  (red), the pre-03 order (`-y`, then distance) 3.32, layer order 3.34. Bound
  3.6.
- Engine, before the plan changed to one final series, so not the verdict:
  teleport, 3 interleaved rounds, one binary with an env switch
  (`/tmp/claude-streaming-impl/ab03r-exp`).

  | Order                    | Settle s | Relight / gen0 | Fill frames |
  | ------------------------ | -------- | -------------- | ----------- |
  | `bin-07` (pre-03)        | 0.66     | 4.84           | 0, 16, 26   |
  | column (as merged)       | 1.13     | 7.13           | 0, 0, 0     |
  | layer (this change)      | 0.73     | 4.23           | 0, 0, 0     |
  | score bands of 2 Cells   | 1.79     | 10.95          | 0, 0, 0     |

  Medians. Layer's ranges did not overlap `bin-07`'s on settle (0.72-0.74
  against 0.62-0.67). The rest of the gap is level-0 generation draining later
  (0.28-0.30 s after the jump against 0.14-0.23), not relight.
- Not taken: holding a Chunk's solve until its six neighbours are held or
  outside `load`. Harness 2.84 with layer order, 3.66 with column. Not run in
  the engine, and it can hold a mesh back on a Chunk two Cells away.
- Re-score cost: `frames.csv` gains `rescore_ms` (profiling and dev builds):
  main-thread time the frame's Server ticks spent rebuilding dispatch orders,
  every level. Read it from the final series.
