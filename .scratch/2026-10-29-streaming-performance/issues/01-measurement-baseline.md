# 01: Streaming fixtures, attribution and baseline

Status: ready-for-agent
Blocked by: none

Spec: [streaming performance](../spec.md).
PR boundary: this branch, `tech/streaming-performance`, with the spec.

## What to build

Make the reported symptoms measurable before anything is optimized. Fixtures
register in `server/profile_fixture.rs` (the name list at line 16),
`scripts/profile-frames` (`--fixtures` choices) and the CSV header in
`profiling.rs`. Like the existing four, each runs 32 s and is measured in its
`fixture` phase, 22 to 32 s.

- Four fixtures on the demo's default seed:
  - `sprint-flight`: noclip at 64 m/s along a fixed heading.
  - `sprint-walk`: physics at 8 m/s along a heading checked to stay on walkable
    ground, clear of cliffs and lakes, for the whole run.
  - `dive`: noclip, straight up 320 m at 64 m/s over 22-27 s, then straight
    back down to the start over 27-32 s. `capture.rs`'s `fly_forward` zeroes
    the heading's y, so this needs its own vertical motion.
  - `teleport`: a 2 km jump at 22 s, recording time to `streaming_settled`.
- Per-frame columns:
  - queue length, dispatched and landed counts per stage (generate, relight,
    mesh) and per level;
  - worker CPU time per stage and level, summed from job thread clocks;
  - drawn Chunks, entities and draw calls per camera;
  - fluid's share of the Server tick, separate from streaming.
- Two hole counts per frame, over level-k Cells inside their mesh shell that
  intersect the camera frustum, computed on the Client from what it was sent,
  never from World storage:
  - *rule holes*: some level has delivered (or, after 02, retains) the region,
    but the draw rule in force discards it. Today that rule is the metre cut,
    so a delivered coarse Cell inside the cut whose finer Chunks are not
    delivered is a rule hole.
  - *fill holes*: no level has delivered the region.
- GPU time per pass: wgpu timestamp queries where Metal supports them,
  otherwise a written Metal System Trace procedure that gives per-pass times.
- A baseline: matched interleaved release series of every fixture, the four
  existing ones included, using `main`'s demo dist (PR 7 merged as #30), pinned
  by file hash. Plus one presented run on the 144 Hz display.
- An attribution checkpoint, appended to this ticket: for each sprint, the
  share of Server tick, main-thread mesh upload and spawn, render CPU, GPU and
  waits in frames over 16.7 ms. Propose any reorder of tickets 02 to 14.

## Acceptance criteria

- [ ] The baseline `dive` series reports rule holes, and a review capture of a
      hole frame shows the hole where the count places it.
- [ ] Every new column stays within the profiler's overhead bound (3% of
      median, 6% of p99). A column over it is opt-in and off in timing runs.
- [ ] GPU time is attributed per pass. If it cannot be, this ticket stays open
      as `needs-info` for you, and 07, 08, 09 and 10 stay blocked.
- [ ] The checkpoint names which sprint stalls and what dominates it.

## Validation and authorization

frame-profiling.md throughout: release with `profiling`, executable hashes,
three or more interleaved rounds, no competing builds, and wait for
`/tmp/tesserae-frame-profile.lock`. Captures stay in one unique `/tmp` folder
and are deleted after review. Local work only; no push or PR without
approval.

## Comments

### 2026-10-30: attribution checkpoint (partial)

Executable `/tmp/tesserae-streaming-bin-b5637df` (sha256 d1a6a0867ff8...),
release with `profiling`, offscreen 1080p, `main`'s demo dist. Series
`/tmp/tesserae-streaming-baseline-b5637df`: three interleaved rounds of the six
other fixtures; `dive` and `teleport` have two, because another session's
`cargo test` started inside `dive-3-on` (moved to `excluded/`). A supplemental
series was refused twice by the same contention. Still owed: their third
round, and the presented run on the 144 Hz display, which needs you at the Mac.

Fixture-phase medians across runs:

| Fixture       | Frame p50 | p95   | p99   | >16.7 ms | >33 ms | GPU p50 | Main p50 |
| ------------- | --------- | ----- | ----- | -------- | ------ | ------- | -------- |
| stationary    | 9.69      | 12.24 | 13.82 | 12       | 1      | 9.36    | 3.26     |
| flight        | 9.69      | 12.14 | 13.50 | 6        | 0      | 9.43    | 3.44     |
| sprint-flight | 9.46      | 12.39 | 14.56 | 12       | 3      | 9.19    | 3.88     |
| sprint-walk   | 9.53      | 11.93 | 12.82 | 3        | 0      | 9.22    | 3.19     |
| dive          | 8.48      | 17.27 | 27.83 | 166      | 21     | 7.78    | 2.54     |
| teleport      | 9.21      | 11.66 | 12.14 | 0        | 0      | 8.96    | 2.71     |

Frame counts over 16.7 and 33 ms are totals over the runs (about 3,100 frames
per fixture).

- **The frame is GPU-bound.** GPU time per frame is within 0.3 ms of frame p50;
  the render thread's wall time matches it, waiting on the GPU, and the main
  thread spends about 3 ms. About 7 ms of the 9.3 ms is the eight cameras' own
  views: each view's depth prepass is 0.2 to 0.8 ms and shadows add about
  0.25 ms on four of them, while opaque and transparent passes are under
  0.1 ms. A view with almost nothing in it still costs about 0.45 ms. About 20%
  of frames lose their last markers, so the GPU columns cover about 80%.
- **Holes appear when streaming falls behind, not from fast flight.**
  `sprint-flight`, `sprint-walk` and `dive` have no hole frames, and neither
  does a `dive` pitched 70 degrees down. After `teleport`, level 0 shows in
  0.65 s, but coarse rule holes last until about +4.4 s: levels 2 to 5 stay cut
  over finer levels that have not arrived.
- **Coarse levels fill strictly one after another.** After the teleport, level 2
  starts only once level 1 has generated its whole load extent (+1.5 s), level 3
  once level 2 has (+3.5 s). Level 1 generates 1,639 Chunks and meshes 564:
  about two thirds of its generation lies inside the region level 0 draws.
- **The 30 fps sprint stall does not reproduce offscreen.** Sprint frames over
  16.7 ms are 0.4% and main-thread bound (median 13.8 ms, Server 4.9 ms).
  `dive` is the worst fixture: 5% of frames over 16.7 ms, with render-thread
  wall time near 15 ms against GPU near 8 ms.
- **Workers are far from saturated.** `sprint-flight` totals about 0.95 s of
  worker CPU per second out of 3 available; level-0 relight is 0.56 s of it.
  Stationary, fluid keeps level 1 regenerating and re-meshing at about 0.32 s
  per second, besides 0.04 s of fluid jobs.

Proposed order, for your approval: 02 (no gaps), 03 (priority), 07 (cameras),
04 (coarse generation, now with the hole-extent finding), then 14 only if the
presented run shows the sprint stall; the rest unchanged.
