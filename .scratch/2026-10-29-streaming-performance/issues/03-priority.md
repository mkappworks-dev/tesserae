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
