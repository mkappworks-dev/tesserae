# 11: Tune the workers

Status: ready-for-agent
Blocked by: 03, 04

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-workers`.

## What to measure

Starting from `StreamingBudget`'s 2026-10-12 table (6 threads made the worst
tick 12.7 ms; 4 was within noise of 3):

- `TESSERAE_ASYNC_THREADS` from 2 to 5, and Bevy's compute/async pool split.
- Coarse jobs yielding to level 0, for example dispatching coarse only while
  level 0's queue is short.
- The in-flight caps, retuned now that order is no longer nearest-first.

Fluid and Mod jobs share these workers; report their share, but do not
optimize fluid here.

## Acceptance criteria

- [ ] Every change adopted passes the spec's effect gate, on fill-hole frames,
      time to `streaming_settled` or frame time.
- [ ] A new pool only if it beats the existing pool.
- [ ] The `StreamingBudget` and `LodBudget` doc tables updated with the result.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.
