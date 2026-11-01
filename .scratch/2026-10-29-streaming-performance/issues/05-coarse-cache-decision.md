# 05: Decide whether to cache coarse levels on disk

Status: ready-for-human
Blocked by: 04

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-coarse-cache` for the fixtures and the
measurement; any cache is a further branch.

## What to decide

After 04, add two fixtures on a Persistent World and measure them:

- `restart`: open a saved World and record time to `streaming_settled`;
- `revisit`: travel 2 km away, wait until settled, return.

Cache coarse levels only if coarse generation takes more than 30% of worker
CPU until settled in either, or `restart` takes more than 3 s to settle.

If the gate passes, the cache is derived data, never authoritative: keyed by a
generator and parameter hash plus the saved-change revisions, discardable at
any time, never written for Ephemeral Worlds. ADR-0011 is amended to say so,
and only then.

If it fails, close as `wontfix` with the numbers.

## Acceptance criteria

- [ ] Both fixtures measured and recorded here.
- [ ] Your decision recorded under Comments.

## Comments

### 2026-11-01: fixtures built; measurement pending the final series

Built on `tech/streaming-coarse-cache`; no cache. Both fixtures are profile
fixtures (`server/profile_fixture.rs`, `scripts/profile-frames`, documented in
`docs/performance/frame-profiling.md`) and refuse to start without a
`TESSERAE_WORLD_DIR` that does not exist yet; the runners give each run one.

- `restart`: at 22 s the World saves and quits through Save and quit, then
  reopens from its save in the same process, the World list's path. Pipelines
  are already compiled, so it times the World opening and refilling, not a
  cold launch.
- `revisit`: jumps 2 km along x at 22 s, returns as soon as the far side has
  settled (seen unsettled, then settled).
- `frames.csv` gains `fixture_moves`. `streaming.json` reports per move:
  `settle_s` from the move's first frame, `blank_s` (frameless gap: the
  reopen's save, teardown and load), worker CPU per level and stage until
  `streaming_settled`, and the same until `coarse_generated` (no coarse level
  has a Chunk of its load extent waiting), each with
  `coarse_generate_share`. `flow.py` prints them when `streaming.json` exists
  (`scripts/profile-frames --analyze-only <ab out>`).

`streaming_settled` is level 0's, and level 0 has the workers first, so the
share "until settled" understates coarse work: at that point levels 1 to 4
have often generated nothing. Both readings are recorded; which one the gate
means is your call.

Smoke runs, one each, `bd97491` (integration `48d9f93` merged), `shot.sh`.
Not a measurement; the gate waits for the final series.

| Run                | Move   | Gap    | Settle | Coarse gen share to settle | Coarse generated  |
| ------------------ | ------ | ------ | ------ | -------------------------- | ----------------- |
| restart            | reopen | 0.27 s | 0.76 s | 22%                        | not by +10.9 s    |
| revisit            | out    | 0      | 0.83 s | 27%                        | not before return |
| revisit            | back   | 0      | 0.84 s | 35%                        | not by +10.5 s    |
| teleport (no save) | out    | 0      | 0.81 s | 15%                        | 4.9 s, share 32%  |

An earlier pair at `9da8869` (before the merge): restart gap 0.30 s, settle
1.15 s, share 15%, coarse generated at 10.6 s with a 62% share; revisit return
settle 1.38 s, share 30%, coarse generated at 6.9 s with a 57% share. Until
settled, level 0's relight is the largest part (620 to 810 ms of 1.2 to 1.9 s).

Finding for the decision: on a Persistent World, coarse generation costs far
more than on an Ephemeral one. Per job, levels 2 to 5 take 1.3, 2.6, 2.8 and
4.6 ms in `restart` against 0.20, 0.19, 0.16 and 0.18 ms in the same binary's
`teleport`, and levels 1 to 4 run about 2,240 jobs for a 1,639-Chunk extent.
The demo's fluids give 41 level-0 Chunks revisions within 22 s (3.2 MB
commit). `lod.rs` then (a) regenerates the covering Chunk at every coarse level
on each level-0 cover change, not only level 1 (line 973), and (b) builds any
coarse Chunk covering a revised Chunk through `generate_persistent`, which
regenerates each relevant child level by level down to level 1. So 04's
per-level costs do not describe a Persistent World, and the restart backlog
(about 2,730 Chunks waiting at +10.9 s) is likely mostly this path. A cache
would hide it; making `generate_persistent` reuse held coarse children might
remove it. Neither is built.

Also: the profiler's origin is taken before `main` inserts `ProcessStart`, so
`frames.csv` times run about 0.13 s behind the clock fixtures move by (the
jump lands at 21.87 s). `settle_s` is anchored on the move's own frame and
unaffected; `ab.py`'s teleport settle (first settled after 22.05 s, minus 22)
carries the offset.

Status stays `ready-for-human`: effect gate and decision pending the final
review series.

### 2026-11-01: gate ruled on; cause fixed first (16); is the cache needed?

You chose to fix the cause first: ticket 16, done on this branch. You also
ruled that the 30% gate is judged on worker CPU until coarse generation has
caught up (`coarse_generated` in `streaming.json`), not until
`streaming_settled`.

After 16, smoke runs at `6782863`, one each (16's Comments have the table):

| Run                    | Settle from move | Caught up | Worker ms | Coarse gen ms | Share |
| ---------------------- | ---------------- | --------- | --------- | ------------- | ----- |
| restart (reopen)       | 0.27 + 0.77 s    | 7.95 s    | 6,351     | 2,427         | 38%   |
| revisit (return)       | 0.76 s           | 4.61 s    | 4,080     | 1,419         | 35%   |
| teleport, unsaved, ref | 0.81 s           | 4.91 s    | 3,562     | 1,148         | 32%   |

Before 16 neither saved run caught up inside the window (backlog 2,876 and
253 Chunks at +10 s; now 2 and 2).

Answer, by the gate as ruled: still yes. Both fixtures are over 30% (38% and
35%); `restart` settles level 0 1.04 s after the quit, well inside 3 s, so only
the share trips it. Two numbers to weigh before building it:

- An unsaved `teleport`, which a disk cache would not touch, is at 32%: once
  level 0 settles, coarse generation is about a third of the work anywhere, so
  the gate now trips on the share any refill has, not on something saved
  Worlds do.
- What a cache could remove on `restart` is the 2,427 ms of coarse generation
  out of 6,351 ms; coarse meshing (2,670 ms) and level 0 stay. Level-0 settle
  would not move. At best it shortens the 7.95 s catch-up by roughly a third,
  at the cost of the derived-data cache and ADR-0011 amendment above.

Smoke runs, not the review series; the gate's numbers come from the final
series. Status stays `ready-for-human`.
