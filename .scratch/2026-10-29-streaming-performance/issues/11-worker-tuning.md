# 11: Tune the workers

Status: done
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

### 2026-11-01: final series; no change adopted

Final series `/tmp/claude-streaming-impl/final`: release `profiling` builds,
offscreen, two interleaved rounds (cut from three), one binary per merge in
order (`bNN` is the integration branch just after ticket NN), the tip `075ff52`,
and the tip with one switch changed. Medians of the two runs; a range is the two
runs, low-high.

- Arms at the tip with `TESSERAE_ASYNC_THREADS` 2 to 5 against Bevy's default,
  in both sprints, `dive` and `teleport`. Frame p50 within 0.1 ms of the tip in
  `sprint-flight`, `dive` and `teleport`. In `sprint-walk` every count beat the
  tip by 5-6%, including the one equal to the default, so that is the tip's run,
  not the count.
- Settle after `teleport` 0.78-0.81 s for every count; fill-hole frames 0. At 5
  threads workers spent about a sixth more CPU on the same `sprint-flight`.
- `wait_for_settled_light` off (`TESSERAE_SETTLE_GATE=0`): no change in frame
  time or settle (0.76-0.78 s against 0.78-0.80).
- No change passes the gate, so none is adopted; no new pool. `StreamingBudget`
  and `LodBudget` docs record the result.
