# Meshing off the tick

Status: done
Blocked by: 04

The most expensive stage and the one the Client sees. Spec decisions 4 (mesh), 6
(settled gate), 7, 8 and 9.

## What to build

- A mesh Job: a Snapshot of the centre and six neighbours, Chunks and light,
  calling the unchanged `mesh_chunk`.
- Dispatch nearest first inside the `mesh` extent, up to the in-flight cap, only
  for a Chunk that is dirty and whose six neighbours exist, **and** whose own
  and six neighbours' light is settled (not `unlit`, no relight in flight).
  Dispatch moves the Chunk out of `dirty`.
- Landing, in the tick, after `retire`: the result is delivered if the Chunk is
  still inside the `mesh` extent, even if it was re-dirtied during the flight,
  in which case it stays dirty. Delivery writes `ChunkMeshReady` and updates
  `delivered`. A result for a Chunk outside `mesh` is dropped and the Chunk is
  marked dirty, so it is redrawn if the anchor returns.
- `retire` cancels mesh Jobs for coords outside `keep`.
- Count, per stage, Jobs landed and results dropped or delivered stale. The
  counters feed `StreamingStats` (dev), so ticket 01's line can report them.
- Rewrite the remaining streaming tests per spec decision 9.

## Measure the gate

Spec decision 6 says the settled gate costs a Chunk's first appearance a relight
it was already waiting for. Report, with and without it, mesh Jobs landed, mesh
Jobs delivered stale, and fill time. Keep the gate unless it costs fill time.

## Acceptance criteria

- A mesh the Client holds at quiescence equals a fresh `mesh_chunk` of the
  store, after a fill, after an Edit on a Chunk boundary, and after a fly-by
  that evicts mid-flight.
- Sustained Edits to one Chunk still deliver meshes.
- No `ChunkMeshReady` is ever written for a Chunk outside the `mesh` extent, and
  none after that Chunk's `ChunkUnloaded` without a `dirty` mark in between.
- At every message, the Chunk and its neighbours were settled.
- The tick does not call `mesh_chunk`, `light_chunk` or `Worldgen::chunk`.
- `./scripts/cargo-ci` passes.

## Comments

What landed: `build_mesh` in `jobs.rs` (a snapshot of the centre and six
neighbours, calling the unchanged `mesh_chunk`); `dispatch_meshes` (nearest
first, only for a dirty Chunk whose six neighbours exist and whose own and
neighbours' light is settled; dispatch takes the Chunk out of `dirty`);
`land_meshes` (after `retire`, delivers if still inside the `mesh` extent even
when re-dirtied, otherwise drops and marks dirty); `retire` cancels mesh jobs
outside `keep`. `publish` is gone: the tick no longer calls `mesh_chunk`,
`light_chunk` or `Worldgen::chunk`. `Stage` counts jobs landed and results
stale, and the capture line reports them.

New tests, mutation-checked: a mesh waits for its neighbourhood to settle (fails
without the gate); sustained edits to one Chunk still deliver (fails if a result
is dropped for having been re-dirtied, after the 60 s wall-clock ceiling); a
fly-by with meshes outstanding delivers nothing outside the new extent, at 2
Cells (between `mesh` and `keep`, where only the check at landing stands in the
way) and at 10 (past `keep`, where `retire` cancels the job).

Test infrastructure, because the tests share one process-wide pool: a job is
held or scrambled by **awaiting** (`Gate`, `yield_now`), never by sleeping,
since a parked worker starves every other test; and `settle` and the other waits
are wall-clock, not tick counts, for the same reason. The first version slept
and failed whenever the suite ran in parallel and passed alone.

**The settled gate, measured, and not yet decided.** Placeholder caps, five runs
each, the same session (median):

| | sync (ticket 01) | gate on | gate off |
| ------------------ | ---------------- | -------- | -------- |
| `shown_s` | 1.10 | **7.75** | 1.22 |
| `fill_s` | 7.24 | 8.08 | 8.16 |
| `worst_fill_ms` | 44.1 | 44.9 | **19.8** |
| `mesh_landed` | - | 1085 | 3892 |
| `mesh_stale` | - | 14 | 561 |

The gate saves 3.6x the mesh work and buys nothing visible: nothing appears for
7.7 s, because the relight backlog (7900 solves through a landing cap of 12 a
tick, the old per-tick count) is the long pole and the gate waits on it. The old
code showed provisional light at 1.1 s and refined it. Without the gate the
worst frame is less than half. So under these caps the spec's rule ("keep the
gate unless it costs fill time") says drop it. But the caps are placeholders and
the relight stage is throttled to the old 12 a tick; ticket 06 sizes them and
re-runs this comparison, and the gate is kept or removed on that.

**The display changed mid-session.** The same commit measured a 6.9 ms median
flight frame (144 Hz) in ticket 01 and 16.7 ms (60 Hz) when re-run an hour
later, and a gate-off run has medians of 8.4 and 16.7 in one batch. Under vsync
a frame time is the refresh interval, so "worst frame" is partly the display's
mood. Ticket 06 turns vsync off for the capture so a frame costs what it costs,
and re-baselines `main` alongside, interleaved.
