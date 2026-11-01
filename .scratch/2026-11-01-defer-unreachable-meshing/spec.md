# Defer meshing Chunks the Eye cannot reach

Status: wontfix.

Source: `tech/streaming-integration` at `fff9788`, not yet merged to `main`.
Evidence: tickets 12 and 15 of
[streaming performance](../2026-10-29-streaming-performance/spec.md).
Tickets: [issues/](issues/).

## Problem

Ticket 12 found that some level-0 mesh worker CPU goes to Chunks that a walk
from the Eye through open space does not reach, mostly sealed caves, and set a
30% bar for acting on it. `sprint-flight` measured 21-26%, under the bar, so
nothing was built. This spec asks again against the spec's effect gate on
worker CPU instead of that bar: if the Server held those mesh jobs back, would
the workers do measurably less, without anything popping in?

## What the numbers say

Two sources, release `profiling`, offscreen 1080p:

- Unreachable share: ticket 12's single runs (`bin-12-5d96f8b`,
  `/tmp/claude-streaming-impl/t12-counts2`), the Server's `reach` walk, level-0
  mesh jobs only.
- Worker CPU per stage: ticket 15's tip (`df93f3b`, two runs,
  `/tmp/claude-streaming-impl/t15all`), `frames.flow.csv` over 22-32 s.

Measured phase, worker CPU in ms per second; ranges are the two tip runs:

| Fixture         | Workers | Level-0 mesh | Its share | Unreachable | Of all |
| --------------- | ------- | ------------ | --------- | ----------- | ------ |
| `sprint-flight` | 820-839 | 36-39        | 4.3-4.6%  | 9.8         | 1.2%   |
| `sprint-walk`   | 530-533 | 30-32        | 5.6-6.1%  | 0.7         | 0.1%   |
| `cave`          | 533-565 | 27-32        | 5.0-5.6%  | 0           | 0%     |
| `stationary`    | 543     | 25           | 4.7%      | 0           | 0%     |

- **The ceiling is at the gate.** Level-0 meshing, reachable or not, is 4.0 to
  6.2% of worker CPU in every fixture at the tip (`dive` 4.0-4.6%, `flight`
  6.2%, `teleport` 5.3-5.5%). Holding every level-0 mesh job would barely pass
  a 5% gate; holding the unreachable ones saves at most 1.2%, in
  `sprint-flight`.
- **`cave` has nothing to defer.** No level-0 mesh job in its measured phase is
  for an unreachable Chunk: the Eye stands still, and what it cannot reach was
  meshed during startup. Its 27-32 ms/s is re-meshing of reached Chunks.
- **Startup is no better.** Over the first 20 s, unreachable Chunks take 4.3 to
  4.8% of level-0 mesh CPU in every fixture: 31-37 ms of 700-800.
- **Ranking saves nothing.** Workers run about 0.8 of their 3 threads in
  `sprint-flight`, so a job moved to the back of 03's order still runs moments
  later. Only holding a job until its Chunk is reached saves CPU, and then only
  for Chunks that unload first.
- **Net of cost, less.** Release would pay connectivity on every level-0
  generation (12: 3.4-13.5 us a Chunk, against 65-93 us to generate it) and
  the `reach` walk on the Server tick, unmeasured outside `profiling`.
- **No target has room to gain.** Ticket 15 met every target at the tip: zero
  fill-hole frames in `dive` and both sprints, settle after `teleport`
  0.78-0.80 s. Ticket 11 ran 2 to 5 worker threads and settle did not move, so
  worker CPU is not what bounds it.
- **The share is highest where the premise is weakest.** `sprint-flight` is
  noclip: its Eye passes through rock, so "not reached through open space"
  does not mean it cannot get there.

## Decision

`wontfix`. Connectivity and the `reach` walk stay a `dev` and `profiling`
instrument. Ticket 01 puts the reason where the next reader will look for it.
Tickets 02 and 03 keep the build plan, also `wontfix`, so a reopen starts from
it instead of from scratch.

Where worker CPU goes instead, at the tip: fluid 37-60% (190-320 ms/s, its own
spec per the streaming spec), coarse levels 100-310 ms/s, and level-0 relight
297 ms/s in `sprint-flight`. Relight of unreached Chunks is not deferrable:
light is World state the Server keeps for every Chunk in the load ring.

## Reopen when

Any of these, measured as above:

- level-0 meshing passes 20% of worker CPU in a sprint fixture, where 12's
  21-26% unreachable share would be about 5% of all of it; for example once
  fluid and coarse costs fall;
- workers saturate: fill-hole frames return in a sprint fixture, or
  `mesh_waiting` grows through the measured phase;
- a World whose unreachable share is far above the demo's, from deeper caves
  or a taller load extent. Re-run `--instruments visibility` on it first.

## If reopened

- **ADR-0002.** Nothing new reaches the Client. Release `ChunkMeshReady` carries
  no connectivity: since 12's cull was removed, only the `profiling` sight count
  reads it.
- **Core.** `tesserae-core` already has `connectivity` and `reach`, free of
  bevy, wgpu and rayon; both stay as they are. Margins are in Cells, which are
  metres (ADR-0003).
- **`reach` is sound for a walking Eye.** A Chunk is marked once a reached
  neighbour's connectivity lets the walk out towards it, so the Chunk under
  open ground, whose top faces are drawn in its own mesh, is always reached.
  Unknown connectivity counts as open. It is not sound for a noclip Eye, nor
  between an Edit that opens a path and the next walk; ticket 03's margin
  covers both.
- **Acceptance**, from your brief, corrected for what `cave` can show: worker
  CPU passes the effect gate in both sprints; zero new fill-hole or rule-hole
  frames; settle does not regress; and no popping in a `breach` fixture, which
  opens a sealed cave in view, in place of `cave`, which has nothing held.

## Tickets

| #  | Ticket                                    | Blocked | Status          |
| -- | ----------------------------------------- | ------- | --------------- |
| 01 | Say why connectivity stays profiling-only | none    | done            |
| 02 | Connectivity and `reach` in release       | reopen  | wontfix         |
| 03 | Hold level-0 meshes of unreached Chunks   | 02      | wontfix         |

## Constraints

- `tesserae-core` stays free of bevy, wgpu and rayon; no Client system reads
  World storage (ADR-0002); everything physical in metres (ADR-0003).
- Measurements follow frame-profiling.md. Every build, test, bench and engine
  run goes through `/tmp/claude-streaming-impl/heavy.sh`; offscreen only.
