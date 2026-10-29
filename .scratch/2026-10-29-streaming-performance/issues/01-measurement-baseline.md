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
  - `dive`: noclip, 320 m above the surface at 22 s, straight down at 64 m/s to
    2 m above the ground, then straight back up, inside the measured phase.
    `capture.rs`'s `fly_forward` zeroes the heading's y, so this needs its own
    vertical motion.
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
