# 03: Carry flow across loaded Chunks deterministically

Status: done
Blocked by: 02

Spec: [decisions 6 and 7](../spec.md).

## What to build

Make streams cross loaded Chunk boundaries, pause at absent terrain, and
resume on admission, with deterministic logical progress and fair bounded
work during concurrent player and Mod Edits.

## Acceptance criteria

- [x] Tests cross all six faces, negative coordinates, and different Chunk
  admission orders. Missing terrain is unknown, never Air.
- [x] Admission wakes both sides; eviction drops local state/work and wakes
  held dependents. A new Chunk at an old coordinate rejects old-lifetime Jobs.
- [x] No fluid-driven terrain loading, offline catch-up, or persistence is
  added. Generated sources return on regeneration under existing semantics.
- [x] Logical steps define when ordered Edits, Mod Commands, admission, and
  eviction enter the simulation. Worker completion order is not that order.
- [x] Equal step-indexed events produce equal canonical simulation state
  under different hash seeds, worker counts, quotas, and scrambled completions.
- [x] Live Edits invalidate affected proposals without globally restarting
  unrelated work, starving player Edits, or landing partial transactions.
- [x] Evaluation, activation scans, wakeups, retained results, in-flight Jobs,
  and landing are count-bounded. Stale/rejected/blocked attempts count as work.
- [x] Queue saturation preserves continuation or explicit backpressure.
  Coalesced per-Chunk activation work is resumable; nothing required is dropped.
- [x] Player Edits, fair Mod queues, and each active Chunk make progress under
  saturation. The two-Voxel Mod drain cap stays unchanged.
- [x] Quiescent fluid and cold unburning terrain perform no recurring
  per-Voxel simulation evaluation.

## Verification and completion

Extend held/scrambled Job tests and the controlled Server-tick harness.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Determinism controls the authoritative logical event trace. Equal wall-clock
  ticks with differently admitted terrain or Mod batches are not equal inputs.
- 2026-10-23: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- `tesserae-core/src/agenda.rs` (new) replaces `flow::Wakeups`. An `Agenda`
  holds one entry per Voxel (the earliest step asked), bounded by `Limits {
  capacity, reserve, scan_per_step }`. A wake that finds it full is not lost:
  it becomes a rescan `Span` of its Chunk, at most 8 spans a Chunk per pass
  (past that the closest pair merges into its covering box, so separate
  clusters stay separate). Rescans are served `scan_per_step` Voxels a step,
  round the Chunks in turn from the one served last, each from a cursor; a
  span added mid-pass waits for the next pass, so nothing the cursor passed is
  missed. `take_due` keeps the step's Voxels' entries until `end_step`, so
  `rewake` (timers, deferrals, stale drops) never overflows; `reserve` entries
  past `capacity` are open to rescanned Voxels only, which is what lets a
  Chunk served by a rescan start a timer while wakes keep the rest full.
  `park`/`unpark` put a Chunk's Voxels to sleep as one span; `forget` drops a
  Chunk's work. Nothing iterates a hash map where order could show.
- `flow`: `Timers` gained `split_off`, `merge`, `hold`, `remove`, `retain`, so
  a step's Voxels and their timers can go to several workers and come back.
  An evaluation that reads unknown terrain now drops its timer: no change
  decided from terrain that is gone lands later (it also left orphan timers
  before). `wake_around(agenda, ..)`, `DEPENDENTS`, and `readers_outside` /
  `reading_outside` (the spans across a Chunk boundary that read into or out
  of it, derived from `READS`) are public.
- `server/fluids.rs`, rewritten around a step under way (`Running`): at its
  start the step takes its due Voxels, snapshots every simulated Chunk they
  read once for the whole step (one shared `Region`), and splits them into
  slices of `EVALUATIONS_PER_JOB`, each with its Voxels' timers, at most
  `JOBS_IN_FLIGHT` (4) out at a time on the existing `Stage` (keyed by slice).
  When every slice is back the step lands whole in that tick: wakeups and
  timers merge back, at most `LANDED_PER_STEP` changes are chosen, the rest
  deferred (timer held ready for the next step), and the next step starts.
- Landing order past the bound: one change per Chunk in turn, starting after
  the Chunk served last, and inside each from a cursor after the last one it
  landed. Stale and parked attempts count towards the bound.
- Staleness: a change is dropped and its Voxel woken for the next step if an
  event of the step's index touched any position it reads, or any Chunk it
  reads is not in the state it was at the Snapshot (evicted, admitted, or a
  new lifetime; absent then and absent now is the same terrain, since the
  kernel reads lazily), or its own Voxel no longer reads `read`.
- Chunk lifecycle: `ChunkStore` journals the coordinates `insert`/`remove`
  touch. A Chunk is admitted when its first light solve has landed; it is
  evicted when it leaves or its lifetime changes. Admission rescans both
  sides of its boundary (`reading_outside` of it, `readers_outside` in held
  neighbours) and, if it holds fluid and is not uniform, all of it once (the
  initial activation scan); none of this happens unless fluid is held in its
  3x3x3 neighbourhood, so dry terrain streams in at no cost. Eviction forgets
  its agenda entries, rescans, timers and landing cursor, and rescans what
  read it, which then pauses.
- A simulated Chunk with a light solve pending is suspended: its changes are
  parked rather than landed or re-evaluated every step, and its parked span
  is rescanned when the solve lands. This replaces ticket 02's per-change
  unlit deferral, which depended on when the light solve happened to finish.
- Journal: `ChunkStore` keeps up to 4096 changed positions per tick, then
  coalesces by Chunk (`Changed { cells, chunks }`); a coalesced Chunk is
  treated as wholly edited and rescanned with what reads it. The landing's
  edited set coalesces the same way past 4096.
- `ChunkCoord` derives `Ord`; `MeshState::unlit_chunks`. Mod drain:
  test-only `PendingCommands::push_places`/`applied`, the `drain` system made
  `pub(in crate::server)`; `MOD_VOXELS_PER_TICK` is still 2.
- `docs/architecture.md` lists `agenda.rs`.

### When events enter the simulation

`now` is the step under way, or the empty step the simulation idles at. An
event takes the index `now` has when it happens, and an event of index `t` is
first read by step `t + 1`. Step `t`'s Snapshot is taken when it starts,
before any event of index `t`, and it lands after all of them, so worker
completion order never decides which events a step saw.

| Event                               | How `simulate` sees it      | Index |
| ----------------------------------- | --------------------------- | ----- |
| player Edit (`edit::apply_pending`) | journal, same tick (earlier | `now` |
|                                     | in `FixedPreUpdate`)        |       |
| drained Mod Command (`mods::Drain`) | journal, same tick          | `now` |
| Chunk admission (first light lands) | lifecycle, next tick start  | `now` |
| eviction, or a new lifetime         | lifecycle, next tick start  | `now` |
| a simulated Chunk's relight starts  | unlit set, next tick start  | `now` |
| or lands (suspend / resume)         |                             |       |

Changes of step `t` that read anything an event of index `t` changed are
dropped and re-evaluated at `t + 1`; nothing else restarts.

### Verification

- Core: 252 lib tests (237 before), meshing 60 unchanged. Server: 304 in the
  `tesserae` binary (294), one ignored as before. Mods: 74, unchanged.
  Workspace 690 (665). Full `scripts/cargo-ci` passed on the tree this
  commit records.
- Agenda tests: coalescing to the earliest step; coordinate order; past
  capacity no more than the bound is held and every woken Voxel is taken by a
  later step; a rescan visits a whole Chunk a budget per step from its
  cursor; three Chunks share the budget and the turn reaches each even at a
  budget of one; the reserve goes to rescans only; a rewake of the step's
  Voxel always fits; parked Voxels cost nothing until released; `forget`
  drops one Chunk's work only; boxes split at negative Chunk boundaries;
  spans per Chunk stay bounded.
- Kernel tests: the Voxels `readers_outside`/`reading_outside` name are
  exactly those reading across the boundary, checked Voxel by Voxel against
  `READS` for a Chunk at negative coordinates; going unknown cancels a
  waiting change and admission resumes the flow; dividing a step into 2, 3
  or 7 slices finishing last-first gives the same state at every one of 300
  steps (with a block and a removal) and the same work; an agenda of 6 + 2
  slows a 0.25 m stream and settles to the same state, holding at most 8.
- Server tests, over the plus fixture (a 5 Voxel shaft onto four walled 9
  Voxel arms) and the existing corridor, 1 m Voxels:
  - faces: the plus placed inside one Chunk and at two straddling, negative
    origins gives the same state at every logical step through a flow and a
    recession, and the same work; between them the streams cross in +x, -x,
    +z, -z and falling through y, so every face of a Chunk is entered or left
    (flow never climbs, so none enters through a bottom face);
  - unknown terrain: with only the source's Chunk held the shaft fills to its
    boundary and nothing appears below; with the next held, the Voxel beside
    the missing east Chunk stays dry (it would fill if unknown were air) and
    nothing loaded terrain; three admission orders, the source's Chunk first
    or last, settle to the all-at-once state;
  - eviction drops the Chunk's work and every timer, the Voxel beside the gap
    stays wet (paused, not dried), readmission refills; removing a generated
    source, evicting and regenerating its Chunk brings it back flowing;
  - a step stalled while the east Chunk is replaced at its coordinate lands
    nothing in the new one (stale count rises), then the stream enters it and
    settles as the reference;
  - determinism: one trace of admissions, evictions, a source placed and
    removed, a block placed, a second source, replayed with divisions 1024 x
    4, 3 x 1 on a one-thread pool, 7 x 8 on four threads and 1 x 2,
    scrambled completion, events applied at a random tick within their step
    (results held back up to three ticks), each run's hash maps keyed afresh
    (std `RandomState` per instance): equal state at every step, equal work,
    landed, stale, deferred, parked and scanned counts. The same holds with
    the agenda at 12 + 4 and two changes a step, where both bounds bite;
  - saturation: three plus streams and the player's corridor against an agenda
    of 16 + 8 (16 scanned a step) and three changes a step, with two Mods
    queueing 40 placements each. Every tick: the agenda holds at most 24,
    at most 4 jobs are out, at most 3 changes land; each Mod queue applies
    exactly one Voxel a tick (the phase 7 rate of two, in turn); a player
    stone placed mid-flood is in the World on the tick it is made, with no
    rejection. Every area changes at least every 64 steps until settled
    (worst seen 22) and each settles where it does unbounded;
  - suspension: with the east Chunk marked unlit nothing lands there, parked
    changes are not evaluated again over 300 ticks, and on its light landing
    the stream settles as the reference;
  - landing choice: with five Chunks of four changes and a bound of three,
    each step lands three different Chunks, each Chunk's in turn, and after
    seven steps every change has landed once;
  - journal overflow: 4352 stone placements and then a source removal in one
    tick; the removal is coalesced by Chunk and the stream still dries;
  - idle: dry Chunks arrive with no evaluation and no scan; a uniform lake
    beside rock is scanned once along its faces (no change lands) then
    sleeps; a settled stream does no work over 500 more ticks, with no timer
    or agenda entry left.
- The ticket 02 test of a landing cap of one per tick became a test of
  division (quota 2 x 1 and 3 x 8, scrambled): same state at every step, same
  work, more worker calls.
- Mutation checks over the agenda, flow and fluids suites, each compiling and
  failing exactly the tests named: admission waking nothing
  (eviction/regeneration, unknown terrain, idle); ignoring Chunk lifetimes at
  landing (new Chunk at an old coordinate); landing in coordinate order instead
  of in turn (landing choice); overflow dropping wakes instead of rescanning
  them (agenda bound and reserve, tiny agenda, saturation); landing in suspended
  Chunks (suspension); waking dry terrain on admission (faces, idle,
  saturation); no activation scan of wet Chunks (eviction/regeneration, unknown
  terrain, idle); keeping a timer on unknown terrain (going unknown); no reserve
  (reserve test); not recording edited positions (ticket 02's older job test);
  ignoring coalesced journal Chunks (journal overflow).
- No Client mutator: every new `FluidSim`, store and agenda mutator is
  `pub(in crate::server)` or core-internal; core gained no dependency and
  `Cargo.lock` is unchanged.

### Deviations, risks and what later tickets inherit

- Two kinds of knob. How a step is divided (`EVALUATIONS_PER_JOB`,
  `JOBS_IN_FLIGHT`, the pool) changes nothing it computes. The agenda's
  `Limits`, `LANDED_PER_STEP` and the journal caps are rules of the
  simulation, like `STEP_S`: under load they decide, deterministically, when
  a Voxel changes, never whether. Ticket 02's `LANDED_PER_TICK` was a
  division knob; it is now per step, because a step landing over several
  ticks interleaves with Edits and an Edit's effect would depend on how much
  had landed. The determinism criterion's "quotas" are read as the division
  knobs; equal bounds are part of an equal setup.
- "Hash seeds": core's maps use std `RandomState`, keyed afresh per instance,
  so every replay runs under different keys; nothing iterates one where order
  shows. No test injects a chosen seed.
- Pausing keeps stored water beside missing terrain (unknown is not air, so it
  neither spreads into it nor recedes from it) and cancels any change it was
  waiting to make. "No fluid may continue to depend on absent state" is read
  as no pending change or landing computed from it.
- The timer-restart risk from ticket 02: under saturation every area kept
  changing (at most 22 steps apart) and settled where unbounded flow does.
  A timer restarts only when a Voxel's wanted state changes, and a finite
  stream's inputs change finitely often; an input changing faster than the
  delay for ever (a Mod toggling a source every tick) keeps that Voxel
  unchanged, by design.
- For ticket 04: the read set lives in `READS`/`DEPENDENTS`; if passage
  through Shapes reads further, `readers_outside` follows from them and its
  test checks it.
- For ticket 06: a `Proposal` is the landing unit and is validated alone. A
  multi-cell reaction must become one unit, chosen and validated whole, so
  the landing bound never splits it.
- For ticket 07: thermal work can share the agenda (one entry per Voxel, one
  set of bounds) or own one with the same `Limits`; its timers travel like
  flow's (`split_off`/`merge`). Parking, the event table and the staleness
  rule apply unchanged.
- For ticket 09: every bound above is unmeasured (capacity 16384, reserve
  4096, 4096 scanned a step, 64 landed a step, 1024 evaluations a call, 4
  jobs, journal and edited sets 4096). Costs to look at: `forget` and the
  timer sweep are linear in the agenda per eviction; a step's `Region` is
  built on the tick from due Voxels x 10 reads; each generated lake Chunk
  that is not uniform costs one 32768-Voxel activation scan on arrival.
- For ticket 10: new nouns (agenda, rescan, parked, admission as first light,
  an event's step index) are not in CONTEXT.md; the durable inventory should
  count running timers as fluid progress needed to resume.
- Not done here and not required: save/load, offline catch-up, flow-driven
  loading (none added); a live capture.
