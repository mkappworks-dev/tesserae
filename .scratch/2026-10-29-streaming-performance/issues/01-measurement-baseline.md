# 01: Streaming fixtures, attribution and baseline

Status: ready-for-human
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
- [x] Every new column stays within the profiler's overhead bound (3% of
      median, 6% of p99). A column over it is opt-in and off in timing runs.
- [x] GPU time is attributed per pass. If it cannot be, this ticket stays open
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

### 2026-10-31: rebased baseline and profiler overhead

The branch was rebased onto `main` at `4247012` (#41). Two fixes first: since
sessions (#39) the profiler's frame systems and the hole count ran before a
World existed and panicked, and now run as session systems (`4aa94df`); and
`LayoutMenu::opened` failed `cargo bench --no-run` (`5f3d2ad`, from #36). The
b5637df series above predates seven merges and is superseded as the before
side; its GPU attribution still stands.

Overhead, from matched on/off rounds (legacy summary median, ms, 3 rounds of
`stationary` and `sprint-flight`):

| Logging            | stationary | sprint-flight | vs off     |
| ------------------ | ---------- | ------------- | ---------- |
| off                | 7.9        | 8.1           |            |
| CSV only           | 8.2        | 8.4           | +3.7%      |
| CSV and hole count | 8.1        | 8.0           | no more    |
| CSV and GPU marks  | 9.6        | 9.4           | +16 to 21% |

The GPU markers were the 19% in the b5637df on/off pairs. They and the hole
count are now opt-in (`--instruments gpu holes`); timing series run with holes
only. CSV logging alone sits just over 3% of median, a cost both sides of an A/B
pay equally; p99 differences were not systematic in three rounds.

New baseline: executable `3bf8f416...` (release, `profiling`, `4aa94df`),
`main`'s demo dist, offscreen 1080p, 3 interleaved rounds, holes counted,
`/tmp/claude-streaming-impl/baseline`. Fixture phase, ms:

| Fixture       | p50 (runs)       | p95   | p99   | max  | >16.7 | >33 |
| ------------- | ---------------- | ----- | ----- | ---- | ----- | --- |
| stationary    | 8.30, 9.01, 8.09 | 11.31 | 12.80 | 26.4 | 5     | 0   |
| flight        | 8.41, 8.26, 8.04 | 10.47 | 11.77 | 29.6 | 5     | 0   |
| active        | 7.82, 8.11, 8.60 | 10.77 | 12.90 | 26.8 | 7     | 0   |
| transparency  | 8.98, 8.61, 8.55 | 11.08 | 12.71 | 43.5 | 11    | 2   |
| sprint-flight | 7.95, 8.74, 8.10 | 11.42 | 13.94 | 34.6 | 16    | 1   |
| sprint-walk   | 8.07, 8.28, 8.39 | 11.58 | 13.80 | 40.7 | 9     | 1   |
| dive          | 7.45, 7.20, 6.89 | 9.84  | 12.21 | 32.5 | 16    | 0   |
| teleport      | 9.06, 8.10, 7.74 | 10.21 | 11.22 | 36.8 | 4     | 2   |

- Only `teleport` has holes: 1,660 rule-hole and 52 fill-hole frames over the
  three runs. Every other fixture, `dive` and both sprints included, has none
  offscreen. Level 0 settles 0.75 to 0.91 s after the jump.
- The first acceptance box is met on `teleport`, not `dive`: a capture 1.2 s
  after the jump shows a band of clear colour at mid distance, and the count
  places 147 rule-hole Cells in that frame. `dive` shows no hole to capture.
- Workers in `sprint-flight`: 0.90 s of worker CPU per second, 0.66 s of it
  level 0 and 0.58 s level-0 relight; that is over ticket 13's 50% trigger.
  `stationary` spends 0.39 s per second on level 1, regenerating and meshing
  around the lake's fluid.
- Still owed: the presented run on the 144 Hz display, which needs you at the
  Mac, and with it the last box.

### 2026-11-01: final series; only the presented run remains

Final series `/tmp/claude-streaming-impl/final`: release `profiling` builds,
offscreen, two interleaved rounds (cut from three), one binary per merge in
order (`bNN` is the integration branch just after ticket NN), the tip `075ff52`,
and the tip with one switch changed. Medians of the two runs; a range is the two
runs, low-high.

- The final series reran every fixture at `b01` (`86d1def`), for 02 to 14 to
  compare against. Rule-hole frames: `teleport` 588 and 599 a run, every other
  fixture 0.
- Owed: the presented run on the 144 Hz display, which needs you at the unlocked
  Mac.
