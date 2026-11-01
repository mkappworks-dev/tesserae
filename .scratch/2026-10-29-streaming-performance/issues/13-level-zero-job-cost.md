# 13: Cut level-0 job cost

Status: done
Blocked by: 01

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-level-zero-jobs`.

## What to build

Only if 01's checkpoint shows level-0 jobs taking more than 50% of worker time
in a sprint fixture; otherwise close as `wontfix` with the numbers.

`StreamingBudget`'s notes give 252 us to generate, 140 us to light and 395 us to
mesh a surface Chunk a tunnel crosses, measured in a debug build at
`opt-level = 1` before slope smoothing; 01's release worker columns replace
them. Profile the stage that dominates in the sprint, mesh and light first,
and the `wait_for_settled_light` trade (a Chunk shows 0.2 to 0.4 s sooner
without it, meshed 2.2 times as often).

## Acceptance criteria

- [x] The trigger evaluated and recorded here.
- [x] If built: identical output, asserted, and a per-Chunk improvement that
      passes the spec's effect gate in the core comparison methodology.

## Comments

### 2026-10-31: trigger, profile and the flood

**Trigger fired.** 01's checkpoint put level-0 jobs at 0.66 of 0.90 s of
worker CPU per second in `sprint-flight` (73%), relight 0.58. The integration
series (`ab-int`, `int`) still has level-0 relight at 615 ms/s against
generate 28 and mesh 44. Per job, from its `frames.flow.csv`: 17.8k relight
solves over the 10 s phase for 3,300 level-0 Chunks generated, 5.4 solves per
Chunk at 290 to 370 us of thread CPU each; a mesh is 230 to 310 us, a
generation 65 to 93 us. The count per Chunk is 03's ordering; this ticket
cuts the cost of one solve.

**Profile.** `/usr/bin/sample` over the fixture phase of `sprint-flight`
(`bin-int2`): relight jobs are 1,084 of 3,290 samples on an async compute
worker, and 97% of that is self time in `tesserae_core::light::solve`, the
flood inlined into it. Queue growth, equality and `boundary_differs` are
under 1% each. The flood was a FIFO over `[u8; 3]` cells that spread a cell
every time it was raised. Seeds are queued face by face, so light through a
side face floods the interior at 13, 12, ... before the sky reaches it and
floods it again at 15.

**Change.** The flood drains one queue per level, brightest first, so each
cell spreads once, and queues `u16` cell indices stepped through per-face
tables. Each level drains in arrival order: last-in-first-out, full sky goes
down a column before its layer and raises the layer's cells to 14 before 15;
in arrival order the harness below ran 3% faster than last-in-first-out.

**Identical output.** A new test lights a roofed Chunk through a hole and a
side face at once and checks every cell against the brighter path; the 56
`light::` tests, the two-solver agreements among them, pass. The A/B below
asserts every solve's output equal.

**Per Chunk, core methodology.** A harness outside the repo lights a streamed
region (8 x 8 columns, 7 layers of the fixture World, 448 Chunks) to a fixed
point, column by column top-down and then across changed faces: 659 solves.
Base and candidate are separate copies of `tesserae-core` (base at 03ec4e6)
with separate targets; thread CPU (`CLOCK_THREAD_CPUTIME_ID`) around each
solve, user-interactive QoS, under `heavy.sh`, interleaved, 5 repetitions
per run. The digest of every solve's light is equal in every run
(`83b6975d7be5ca76`).

| Side | Run medians, ms of solve CPU | Per Chunk |
| ---- | ---------------------------- | --------- |
| base | 64.89, 65.04, 64.56          | 144.8 us  |
| cand | 43.63, 44.09, 43.31          | 97.4 us   |

-33%, ranges far apart. Criterion `light_chunk` (wall time, 3 interleaved
rounds, medians):

| Row                   | base, us | cand, us | Change |
| --------------------- | -------- | -------- | ------ |
| straddling            | 175.2    | 153.8    | -12.2% |
| straddling_settled    | 290.6    | 176.3    | -39.3% |
| straddling_shaped     | 148.5    | 134.4    | -9.5%  |
| straddling_water      | 133.6    | 89.7     | -32.8% |
| straddling_unsmoothed | 86.5     | 81.7     | -5.6%  |
| leaves_canopy         | 173.8    | 91.9     | -47.1% |
| ramp_densest          | 192.3    | 168.5    | -12.4% |
| flooded_air           | 119.0    | 112.8    | -5.1%  |
| cave_mouth            | 101.4    | 96.7     | -4.6%  |
| cave_dark_neighbours  | 41.4     | 40.8     | -1.4%  |

`opaque` and `open_sky` answer before the flood and stay at 4 ns. No row
regressed and no row's run range overlaps the other side's.
`straddling_settled`, the case the relight budget is sized from, is the
streaming one: every face delivers something.

**`wait_for_settled_light` left as it is.** It gates meshing, 44 ms/s in
`sprint-flight` against relight's 615; off, the budget note's 2.2 times as
many meshes would add about 50 ms/s to show a Chunk 0.2 to 0.4 s sooner. Not
measured here.

**Pending.** The frame and worker-time effect in the engine waits for the
combined review series; the per-Chunk gate above is met.

