# Off-thread chunk streaming

Off the build order: DESIGN.md's ten steps do not list it, and it appears only
as a deferred move in the step 3 and step 4 notes. A restructuring, not a
feature, so the branch is `tech/off-thread-streaming` and takes no `phase-N-`
prefix.

Move the three stages `server/streaming.rs::stream` runs on every `FixedUpdate`
tick - generation, light solving and meshing - onto Bevy's
`AsyncComputeTaskPool`. The tick keeps what only it may do: deciding what to
work on, handing out immutable snapshots, and applying results to World state.

Started after step 5 (shaped voxels, merged as `047725f`) on purpose: step 5
changed `publish` (a second mesher pass) and `relight` (per-Shape opacity), and
a concurrency change and an algorithm change are not to be debugged together.

## Order against the caves task

The user settled it: step 5, then this, then caves. The trade, in a sentence:
doing this first removes caves' hardest constraint (they break the three fast
paths that make a vertical streaming extent affordable, and those matter far
less once the work is off the frame-critical path) at the price of tuning
against today's cheaper workload, which this spec compensates for by stating the
cost assumptions it makes (see "What this assumes about per-Chunk cost") so the
caves work can check itself against them instead of rediscovering them.

Concretely, this plans against `main` plus step 5 with caves **not** landed:
`Worldgen::chunk`'s deep early-out still returns a uniform stone Chunk in
nanoseconds, `light_chunk`'s opaque path still answers from `uniform()` alone,
and `mesh_chunk`'s enclosed case still declines. Most of a 3D ring is nearly
free, and the per-Chunk costs below are the surface band only.

## Why

The tick is not the problem it was: step 4's worst tick was 8.6 ms, and step 5
left it at 8.1 ms (`StreamingBudget::default`'s comment). The problem is that 8
ms of a 15.6 ms tick is spent on one thread that also draws the frame, and the
budget is a count sized for the worst case, so a frame that runs two or three
catch-up ticks after a slow frame pays it two or three times. Step 4 measured
39-48 ms worst frame during the initial fill, from a tick that is "within
budget". The counts cannot be tuned lower without slowing the fill, because the
work is on the frame-critical path. Moving it off removes the trade.

## Vocabulary this task adds

Three terms, to be added to `CONTEXT.md` in the same PR.

- **Job** - one Chunk's worth of one stage (generate, solve light, mesh), run on
  a worker from a Snapshot and returning an owned result. A Job never touches
  World state.
- **Snapshot** - the immutable view of World state a Job is handed: the Chunks
  and light it reads, as shared references.
- **Landing** - the Server applying a finished Job's result to World state, on
  its tick. The only way a Job's work becomes part of the World.

## Baseline

`cargo bench -p tesserae-core` on `main` (`047725f`), Apple M5, 2026-10-12:

| Stage              | Case                      | Cost    |
| ------------------ | ------------------------- | ------- |
| `Worldgen::chunk`  | surface-crossing          | 98 us   |
| `Worldgen::chunk`  | early-out (air or stone)  | 3.6 ns  |
| `light_chunk`      | surface-crossing, settled | 139 us  |
| `light_chunk`      | open sky / opaque         | 4 ns    |
| `mesh_chunk`       | surface-crossing          | 342 us  |
| `mesh_chunk`       | declined (air, enclosed)  | 6 ns    |
| `Neighborhood::new`| the padded copy, isolated | 25 us   |

The last row is the price of the alternative to decision 3 that copies the data
on the main thread; it is in the table because it is the number that decides it.

Worst tick, every Chunk in every budget straddling the surface: 24 x 98 us + 12
x 139 us + 12 x 342 us = 8.1 ms against 15.6 ms at 64 Hz.

The frame picture is re-measured by ticket 01 with a harness that separates the
initial fill from flight, because the existing report folds them into one
number. Step 4's figures, for orientation only: steady state 120 fps, 2167
Chunks loaded and 791 meshed within 2 s, worst frame during the initial fill
39-48 ms over three runs (debug at `opt-level = 1`, so a lower bound).

## Decisions

### 1. All three stages move, as one Job per Chunk

Generation is the cheapest per Chunk and a pure function of the seed, the Voxel
size, the generator parameters and the coord. Meshing is the most expensive and
a pure function of a Snapshot. Light is the one with cross-Chunk dependencies,
and decision 5 argues it is safe anyway. They do not have to move together, and
the tickets land them one at a time in that order, each leaving the tree
correct; but the end state is all three, because leaving one on the tick leaves
the worst-case tick it sets (light's 12 x 139 us is 1.7 ms; generation's 24 x 98
us is 2.4 ms) on the frame-critical path for no gain in simplicity once the
machinery exists.

One Job per Chunk, not batches. The cost of a spawn is a task allocation and a
queue push plus a wake; ticket 03 measures it. If it is a meaningful fraction of
the nanosecond early-out cases (most of a ring), the stage tries the early-out
inline on the tick first and spawns only for the Chunk that needs work. That is
an optimisation the measurement decides, not a design assumption, and it is the
one that stops mattering when caves remove the early-outs.

What stays on the tick: `retire`, the scan that picks what to dispatch, Snapshot
construction, Landing, `Edit` application, and every `Message` write. None of it
is proportional to the cost of a Chunk, which is the property decision 6 leans
on.

### 2. The concurrency lives in `tesserae`, on `AsyncComputeTaskPool`

`tesserae-core` stays at **zero dependencies**. `rayon` is not added.

Everything core already exposes is a pure function over borrowed data -
`light_chunk(&Chunk, [Option<LitChunk>; 6], &ShapeTable)`, `mesh_chunk`,
`Worldgen::chunk` - and `Chunk`, `ChunkLight`, `ShapeTable`, `Worldgen` are
plain owned data, so they are `Send + Sync` already. Core needs no change at
all, which is also the cleanest argument that this task cannot have introduced
an engine edge into it. The boundary check in CI is untouched.

Why not `rayon` in core, although DESIGN.md's crate layout lists it:

- Its useful verbs are fork-join. `par_iter` blocks the caller until the work
  finishes, and the caller here is the thread that draws the frame. Avoiding
  that means `rayon::spawn` plus a channel, which is `AsyncComputeTaskPool` with
  a second pool beside it.
- A second pool is a second set of threads. Bevy already sizes its pools against
  the core count; a global rayon pool on top oversubscribes a 10-core machine
  running a render thread and an ECS executor.
- Core would have to depend on a scheduler to be benchmarkable, when it is
  benchmarkable precisely because it has no opinion about one.

`rayon` stays a candidate for work that is data-parallel *inside* a Chunk or
across a whole LOD level (step 6's downsample), where it would be called from a
Job and not from the tick. DESIGN.md's layout line is amended in ticket 07 to
say so, so the document stops promising a dependency nothing uses.

`AsyncComputeTaskPool`, not `ComputeTaskPool`: the latter is what the ECS
executor and rendering extraction share, and a 342 us Job on it competes with
systems that are on the frame's critical path. By Bevy's default policy this is
25% of cores, clamped to 1-4: **3 threads on this 10-core machine**
(`TaskPoolOptions`, `bevy_app-0.20.0`). Ticket 06 measures 2, 3 and 4 and sets
the count in `main.rs` if it matters.

### 3. Data reaches a worker as `Arc` Snapshots, copy-on-write

`ChunkStore` changes from two maps holding `Chunk` and `ChunkLight` by value to
the same two maps holding `Arc<Chunk>` and `Arc<ChunkLight>`. A Snapshot for a
mesh Job is seven `Arc<Chunk>` and seven `Arc<ChunkLight>` clones: 14
reference-count increments on the tick, no copy. The Job builds `LitChunk`s over
them and calls the unchanged core function.

Mutation goes through `Arc::make_mut`, which clones only if a Job is holding the
`Arc`. An Edit to a Chunk nobody is meshing mutates in place, as today; an Edit
racing a Job pays one Chunk clone (typically ~16 KiB of palette and indices for
terrain, plus up to 32 KiB of dense light) once. `ChunkStore::get` and `light`
keep returning `&Chunk` and `&ChunkLight`, so no read site changes; the mutators
change body, not signature.

Evaluated and rejected:

- **Clone seven Chunks and seven `ChunkLight`s per Job.** Measured by ticket 02
  rather than asserted: 5.6 us per mesh Job against 43 ns for the `Arc` clones,
  130x. That is affordable (67 us a tick at 12 Jobs, 0.6 ms at 100) and was not
  the reason to choose `Arc`; the reasons are that it is free, that it makes
  "unchanged since the snapshot" a pointer comparison, and that the copy grows
  with palette size where the `Arc` does not. The first draft of this spec
  estimated the copy at ~4 MiB a tick; terrain Chunks are small, and it is the
  224 KiB of dense light that dominates.
- **Copy only the six face planes** (a halo: 6 x 1024 Voxels and light). Cheaper
  to hand over, but `light_chunk` and `mesh_chunk` take whole `LitChunk`s, so it
  needs a second entry point in core or a rebuild of `Neighborhood` on the tick,
  and `Neighborhood::new` alone is 25 us per Job - the cost moves, it does not
  go away. It is the fallback if `Arc` COW turns out to cost more than expected,
  and the reason the Job functions in ticket 03 onwards take a Snapshot struct
  rather than the store.
- **Lock the store.** A `RwLock` held by a worker for 342 us stalls every Server
  write for that long. The point is to remove waits.

The ADR-0002 argument, which is the reason this is the right shape rather than
merely a cheap one: a Job receives `Arc<T>`, which grants `&T` and nothing else,
and returns an owned value. There is no `&mut` into World state anywhere on a
worker. Landing is a Server system calling `pub(in crate::server)` mutators.
`ChunkStore`'s privacy stays the whole enforcement, and the new Snapshot
accessor is `pub(in crate::server)` too: an `Arc<Chunk>` in Client hands would
be read access, which the Client is deliberately denied.

### 4. A result is validated before it lands, by state the tick already keeps

`MeshState` already records "this Chunk's light is stale" (`unlit`) and "this
Chunk's mesh is stale" (`dirty`). A Job in flight is a third state, and the
validity rule is that dispatch moves a Chunk out of the stale set into the
in-flight set, so **anything that marks it stale again while the Job runs is, by
construction, recorded for free**. No version counters.

Every result, before it lands:

- **Generate.** Valid if the coord is inside the `evict` extent of the *current*
  anchor and not in the store. Otherwise dropped. Dropping the `Task` is the
  cancellation; a future with no await points runs to completion anyway, so the
  waste is bounded by the in-flight cap, not unbounded.
- **Relight.** Valid if the Chunk is still in the store, is the same instance
  (see below), and no Edit has touched any Chunk the Job read since dispatch.
  Otherwise dropped and the Chunk is marked `unlit` again. A neighbour's relight
  landing during the flight is *not* a reason to drop - decision 5.
- **Mesh.** Landed if the Chunk is still inside the `mesh` extent. If it was
  marked dirty during the flight, the result still lands and the Chunk stays
  dirty, so a fresh Job follows. Delivering a mesh one change behind is
  invisible next to the mesh the Client already holds, and dropping it would
  starve a Chunk under a sustained stream of Edits, because each Job would be
  invalidated by the next one's dirtying before it landed.

**Same instance.** A Chunk can be evicted and regenerated at the same coord
while a Job for the old one runs. Landing checks `Arc::ptr_eq` between the
Snapshot's centre `Arc<Chunk>` and the store's current one. A COW mutation
allocates a new `Arc` whenever a Job holds the old one, and a held `Arc` cannot
have its address reused, so pointer equality is exactly "unchanged since
dispatch" with no ABA hazard. This is the check that makes `Arc` Snapshots free
of counters.

**Edits.** `edit::set_voxel` already reports every Chunk it wrote (the edited
one, the boundary neighbour, and everything `LightLattice::touched` lists). It
calls `MeshState::edited(coord)`, which gains a second job: record the Chunk and
its six face neighbours as *poisoned*, because a relight Job for any of them
read a plane the Edit just changed. The incremental path is not monotone
(removing a light darkens), so a stale bulk result landing over it is not the
benign case of decision 5; it is the bug `tesserae_core::relight`'s doc comment
describes. A poisoned in-flight relight is dropped at landing. Dispatch clears
the poison. `MeshState::is_unlit`, which an Edit waits on before applying, also
answers true for a Chunk with a relight in flight, so an Edit never runs
concurrently with a Job whose *centre* it writes.

### 5. Light is a relaxation; the reasoning, checked

The brief asks for this to be verified rather than taken on trust. The argument
and what checks it:

`light_chunk` is a pure function of a Chunk's Voxels and its six neighbours'
boundary light and Voxels. Each of its paths is monotone non-decreasing in the
neighbours' light: the opaque and open-sky answers are constants or
`max`-with-15, "nothing to spread" returns dark only when every border is dark,
and the flood propagates `max(neighbour - attenuation)`. Per-Shape opacity (step
5) changes which faces a step crosses, a fixed property of the Voxels, not of
the light, so it does not break monotonicity.

The bulk path starts every Chunk dark and only ever replaces a Chunk's light
with `light_chunk` of some snapshot of its neighbours. Neighbours' stored light
is itself non-decreasing over time (induction), so a later Snapshot dominates an
earlier one, so each Chunk's stored sequence is non-decreasing and bounded above
by the least fixed point. A stale read therefore yields a value that is too low,
never wrong in the other direction, and the Chunk is re-marked `unlit` when the
plane it reads next moves (`boundary_differs`, unchanged), so it is re-solved
from the newer plane. Any state at which no Chunk is stale or in flight is a
fixed point of the operator that the sequence stays below, so it *is* the least
one. The result is therefore the same ChunkLight the synchronous order produces,
whatever order the Jobs land in. Two Chunks solved concurrently each reading the
other's pre-write plane costs an extra pass, not a wrong answer.

The one fact this needs from implementation: marking a neighbour `unlit` while
that neighbour has a Job in flight must survive the Job's landing. It does,
because dispatch *removes* from `unlit` and Landing never re-inserts or clears
it; the mark made during the flight is simply still there.

What does not satisfy the argument, and is handled separately: Edits (decision
4, poisoning), and eviction followed by regeneration, which resets a Chunk to
dark - a decrease, but the regenerated Chunk's neighbours hold values derived
from the same Voxels, so they remain at or below the fixed point, and the
synchronous code has the identical reset.

**Checks, not assertions.** Ticket 04 adds a test that runs the relaxation to
quiescence with each Job delayed by a coord-derived pseudo-random time, so
landing order is scrambled, with in-flight caps of 1 and unbounded, and asserts
two things: every loaded Chunk's stored light equals `light_chunk` of its
current neighbours (it is a fixed point), and it equals what the old synchronous
`relight` stage produced (it is the same one). A scrambled order that failed to
converge, or converged elsewhere, is a refutation of this section.

### 6. Count the work in flight and the work landed, not the work started

The old budget is one count per stage per tick, "deterministic ... sized for the
worst case rather than discovered at runtime". It bounded two different things
with one number: the frame cost of running a stage, and how much was done. Going
async separates them, and they want different mechanisms.

**Worker-side cost is bounded by an in-flight cap per stage**, not a per-tick
count. A Job outstanding is a Snapshot held, a queue slot occupied, and a
priority decision that can no longer be revised (the pool is FIFO; an anchor
that moves cannot reorder what is already queued). The cap bounds all three.
Dispatch simply fills up to the cap each tick, in the same order as today:
nearest first for generation and meshing, highest first for light, because sky
light flows down.

**Main-thread cost is bounded by a landing cap per stage per tick**, a count,
for the reason the old comment gave: it is deterministic and testable. It is a
count that can be sized *well* now, because the main-thread cost of a Job is
independent of the Chunk's cost - an `Arc` clone and a hash lookup to dispatch,
a hash write plus a message to land - except for the part that scales with
output: the `ChunkMesh` a landing hands the Client, whose upload
(`chunk_mesh_to_mesh` plus `Assets<Mesh>::add`, in `Update`) is the one piece of
this pipeline still on the thread that draws the frame. The mesh landing cap
exists to bound that, and ticket 06 measures its per-mesh cost to size it.

`ChunkLight` equality and `boundary_differs` (a 32768-Voxel compare, tens of
microseconds for a dense Chunk) move into the Job: it is handed the Chunk's
current light in its Snapshot and returns the new light, whether it changed, and
which of the six faces' planes moved. Safe because only a Chunk's own relight
Job writes its light between dispatch and landing, and an Edit that did is
poisoned (decision 4).

**What the cap is not.** It is not a throughput limit. A cap of 64 mesh Jobs
costs 64 x ~15 us = ~1 ms of main-thread time per tick however expensive each
Chunk is. A sizing error now shows up as workers idle or as stale priorities,
both of which are latency, not frame time. That is the real trade made here, and
it is the trade the PR states: **the cost of a bad number moved from frame time
to fill latency.**

`StreamingBudget`'s doc comment is rewritten accordingly (ticket 06). Its old
claim about relight being half of generation - that the relight *backlog* costs
nothing visible - was a statement about frame cost; with relight off the tick it
no longer sizes anything, and the comment says so rather than keeping a number
whose reason is gone.

**Mesh dispatch waits for a settled neighbourhood.** Today `publish` meshes a
dirty Chunk as soon as its six neighbours exist and have light, even while
relight is still moving that light, and re-meshes when it settles; step 3 and 4
both measured that as acceptable because it was inline. Off-thread, a mesh Job
dispatched against light that is about to change is a Job whose result lands
stale (above: delivered, but then redone). So a mesh Job is dispatched only when
the Chunk and its six neighbours are neither `unlit` nor relighting. This also
strengthens `nothing_is_published_before_its_light_is_solved` from "has light"
to "light has settled". It delays a Chunk's first appearance by the relight it
was already going to wait for; ticket 05 counts mesh Jobs landed and re-dirtied
(the waste) before and after and reports fill time with the gate, so it is
measured rather than assumed.

### 7. The Client sees the same messages, from the same place

`ChunkMeshReady` and `ChunkUnloaded` are still `Message`s, still written only by
the Server's tick system, so the ordering `server/mod.rs` documents is
untouched: `RunFixedMainLoop` runs before `Update`, and a message written on a
tick reaches the Client that frame. **A worker never writes a Message.** A
finished Job sits in its `Task` until the next tick polls it
(`bevy_tasks::futures::check_ready`), and Landing is where the message is
written. Results never arrive "from the pool" in any sense the Client could
observe.

Two consequences, both accepted:

- A frame that runs no fixed tick (at 120 fps against 64 Hz, a large fraction)
  delays a finished mesh to the next tick: up to one tick (15.6 ms) of added
  latency from Job completion to the Client. The alternative, polling in a
  per-frame system, would put the Server's Landing on the Client's schedule
  rather than the tick that a network transport (step 10) will drive, which is
  exactly the coupling ADR-0002 exists to avoid.
- `retire` runs first in the tick, so a coord whose mesh the Client is told to
  drop can never be re-delivered by a result landing later in the same tick:
  Landing checks the `mesh` extent against the same anchor `retire` used.

A panic in a Job surfaces on the tick that polls its `Task`, loudly, rather than
leaving a Chunk silently never streamed.

### 8. The all-six-neighbours gate holds, and eviction needs no re-mesh

The gate is applied at dispatch, as now: a Snapshot of the centre and all six
neighbours, or no Job. The module doc's argument for why eviction never leaves a
stale face - `evict` is wider than `load`, which is wider than `keep`, so an
evicted Chunk is never adjacent to one the Client holds a mesh for - is an
argument about the *extents*, and the extents do not move.

What a Job in flight adds is a window between dispatch and landing in which the
anchor moves. The argument still closes: a Job meshing X reads neighbour Y; for
Y to be evicted before X lands, Y must leave `evict`, so X, which is adjacent to
Y, is outside `load` shrunk by one Cell, which is outside `keep`, which is
outside `mesh`. Landing's `mesh`-extent check drops it. So **a mesh lands only
if every neighbour it read is still loaded**, which is the module doc's
invariant stated for the in-flight case, and the new test for it is the one that
teleports the anchor with Jobs outstanding.

### 9. The test suite keeps its invariants and stops assuming synchronous ticks

The streaming tests drive `stream` with `run_system_once` and assert on what one
tick produced. With Jobs, one tick produces dispatches, and results land on a
later tick. The honest rewrite, per the brief, is a harness that **pumps ticks
until quiescent** (no Job in flight, nothing newly dispatched, nothing to land),
with an iteration ceiling so a livelock fails a test rather than hanging CI, and
asserts on the quiescent state and on every message seen along the way.

- `one_tick_fills_the_load_volume_and_meshes_only_what_has_neighbours` becomes:
  Same assertions after settling, over every message seen on the way. The load
  volume is exactly the stored set; no mesh is sent without its six neighbours.
- `no_mesh_the_client_still_holds_is_stale`: Unchanged in spirit and stronger:
  small in-flight caps over many ticks, and at quiescence every held mesh equals
  a fresh `mesh_chunk` of the store.
- `moving_the_anchor_loads_ahead_and_unloads_behind`: `retire` is still
  synchronous, so the unload assertion is unchanged and runs after one tick;
  "streamed in ahead" runs after settling.
- `crossing_a_cell_boundary_does_not_thrash`: Unchanged, plus: no Job is
  dispatched for a Chunk already held.
- `the_budget_caps_work_per_tick` becomes:
  `in_flight_never_exceeds_the_cap_and_landings_never_exceed_theirs`: asserted
  after every tick. "Dispatched this tick" is a counter the Server keeps, so it
  is deterministic whatever the workers do.
- `nothing_is_published_before_its_light_is_solved`: Stronger: at every message,
  the Chunk and its six neighbours were settled, not merely lit.
- `a_relight_that_changes_nothing_does_not_republish`: Settle, then tick again:
  zero dispatches and zero messages.

New tests, one per hazard decisions 4, 5 and 8 name:

- A Job in flight for a Chunk the anchor then leaves behind lands nothing.
- Relight in scrambled landing order reaches the same fixed point as the
  synchronous stage (decision 5).
- An Edit with relight and mesh Jobs outstanding around it: the relight result
  is dropped, and the held meshes at quiescence equal fresh ones.
- Sustained Edits to one Chunk still deliver meshes (the starvation case).
- A Chunk evicted and regenerated at the same coord while its Job runs: the old
  result does not land on the new instance.

The harness runs on the real `AsyncComputeTaskPool`, initialised once per test
process with a fixed thread count, so these are not timing-dependent: every
assertion is either about quiescent state or a bound that holds whenever it is
sampled.

## Invariants, and where each is enforced

1. **The Server is the sole authority over World state.** `ChunkStore`'s
   mutators stay `pub(in crate::server)`, and so does the Snapshot accessor. Job
   functions take Snapshots and return owned values; no function a worker runs
   has a `ChunkStore` or `MeshState` in its signature. Reviewed against the
   diff: grep for `ResMut<ChunkStore>` outside `crate::server`.
2. **`tesserae-core` acquires no dependency.** Not `bevy`, not `wgpu`, and this
   task adds not even `rayon`. CI's `Boundaries` job still runs; `cargo tree`
   for `tesserae-core` must print one line.
3. **Everything physical is in metres.** Nothing here introduces a distance. The
   budget is counts of Jobs, not time. If ticket 06 finds a time-based window is
   needed it is in seconds.
4. **No Mod reaches `tesserae-core` directly.** Untouched: Jobs call core from
   `tesserae`, and nothing here gives a Mod a handle to a Job or a Snapshot.

## Performance

Measured in frame time and its variance, not throughput. Steady state is already
120 fps and is not the point.

- **Method.** Ticket 01 extends the `dev` capture to report, in one line, the
  time to fill (until nothing is stale or in flight), the worst frame before
  that, and the worst and 99th-percentile frame after it during flight. Both
  `main` and the branch are run five times each; the PR reports the median and
  the range, because step 4's worst frame swung 65-115 ms across three runs of
  one build.
- **Success.** A lower and flatter worst frame during the initial fill and
  during flight, at a fill time no worse than the slowest baseline run.
- **Also reported:** Jobs landed per stage, results dropped as stale per stage,
  and workers' busy time, so a "faster" claim is attributable. Async is assumed
  to be faster and has to be shown to be; if the numbers say the gate in
  decision 6 or the thread count costs fill time, the PR says so.

## What this assumes about per-Chunk cost

For the caves work to check its design against, before it breaks the fast paths.
The new mechanism assumes:

1. **Main-thread cost per Job is independent of the Chunk's worker cost.**
   Dispatch is O(14) `Arc` clones; Landing is a hash write, a message, and a
   move of the result. Caves can make a Chunk 100x more expensive to mesh and
   this does not change. What does grow with caves is mesh *size*, which grows
   the Client upload; the mesh landing cap bounds it, and caves should
   re-measure that cap rather than the Job cost.
2. **The staleness window is `cap x mean Job cost / threads`.** With the caps
   ticket 06 sets, ~128 Jobs in flight, 3 workers and today's surface-band mean
   (a few hundred us at worst, nanoseconds for most) this is well under one
   tick. With caves making the mean 300 us it is ~13 ms, still under a tick. At
   ~1 ms it is ~43 ms, about three ticks, and priorities start to lag a
   fast-moving anchor. That is the point at which the caps want to come down, or
   become a worker-time window (an EMA of measured Job durations, in seconds)
   instead of a count. This spec does not build the latter; it is the named next
   step, and the Job result already carries enough to measure.
3. **Fill time becomes worker-bound.** At 300 us a Job and 3 workers the pool
   supplies ~156 Jobs per tick for every stage together, so a ring of 2167
   Chunks fills in a few hundred ms of wall time at best. Today's fill is
   bounded by the caps, caves' fill by the workers; the cap sizing in ticket 06
   is therefore tuned for the first and is expected to be loose for the second.
4. **The inline early-out optimisation (decision 1) disappears.** If ticket 03
   adds it, caves make it dead weight; it must be easy to delete.

## Known loose ends, declared rather than discovered

- **The Client's mesh upload is still on the render thread.** Converting
  `ChunkMesh` to a Bevy `Mesh` could move into a Client-side task. Not done
  here; ticket 06's measurement says whether it is worth doing next.
- **The scan is still O(load volume) per stage per tick** (`generate`, `relight`
  and `publish` walk the offset lists and test set membership). ~2000 lookups,
  tens of microseconds. A pending queue would remove it; not worth a change
  while it is invisible in a frame.
- **One FIFO pool serves all three stages.** A burst of generation Jobs queues
  ahead of meshing. The caps bound the delay and there is no stage priority; if
  measurement shows meshes starved during a fill, a second pool is the fix and
  is a small one.
- **The `ShapeTable` is shared by `Arc` and immutable after the World opens.** A
  Mod registers Shapes at load, before any Job exists. If Shapes ever become
  registrable at runtime, in-flight Jobs hold the old table and need a table
  epoch checked at Landing; flagged here so that change finds this note.
- **`dev` builders** (`lightbox`, `gallery`, `builder`) place through
  `edit::set_voxel` after waiting for light; they read `MeshState::is_unlit`, so
  they pick up the in-flight case through decision 4. Ticket 04 audits each.
- No LOD, no save/load, no transport. Nothing here changes what a Chunk is.

## Done when

`cargo dev` shows a lower and flatter worst frame during the initial fill and
while flying, at the same or better fill time, over five runs each against
`main`; `./scripts/cargo-ci` passes in full; the streaming tests still assert
the invariants they were written for; and the PR says what replaced the
deterministic per-tick count and what was given up for it: determinism of *when*
work lands, in exchange for a frame time that no longer depends on how much work
there is.


## Outcome

Where the build departed from, or settled, what this spec said. The tickets'
Comments carry the numbers.

- **Decision 3's cost of the copy was wrong.** Cloning seven Chunks and lights
  is 5.6 us, not hundreds. `Arc` stayed for the reasons that survived: it is
  free, and it makes "unchanged since the snapshot" a pointer comparison.
- **Decision 1's inline early-out was not added.** A spawn costs the tick 0.7 us
  against a 3.6 ns early-out, and 38 us a tick at 48 jobs.
- **Decision 4, mesh landing**, held as written: a re-dirtied result is
  delivered, and it was needed (a test fails without it).
- **Decision 5 held**, and each part of it is now a test that fails when the
  part is removed. One of them (a mark made during a flight surviving the
  landing) is not caught by the scrambled-order test and has its own.
- **Decision 6's settled gate is kept**, after looking like a mistake at
  placeholder caps (nothing visible for 7.8 s). Its cost once relight is not the
  long pole is 0.2-0.4 s to first appearance; its saving is 2.2x fewer meshes.
- **Measurement changed.** Wall-clock frame time under vsync measures the
  display, which changed refresh rate on the same commit within an hour. The
  harness reports `MainThreadCost` (the tick and the mesh upload) instead, and
  frame time as a secondary figure.
- **The Client upload is 6-12 us a mesh**, so the spec's loose end about moving
  it off the render thread is closed: not worth doing.
- **More async threads hurt.** Six on this ten-core machine took the worst tick
  from 0.7 ms to 12.7 ms.
- **Flight had nothing to win.** The baseline's flight is flat, and the task's
  "lower and flatter while flying" can only be "not worse".
