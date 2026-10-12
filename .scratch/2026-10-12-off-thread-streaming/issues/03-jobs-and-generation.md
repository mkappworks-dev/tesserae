# The Jobs resource, and generation off the tick

Status: done
Blocked by: 02

The machinery every later stage reuses, proved on the simplest stage. Spec
decisions 1, 2, 4 (generate), 6 and 9.

## What to build

- `Jobs`, a `pub(in crate::server)` resource holding the in-flight `Task`s keyed
  by coord, per stage, over `AsyncComputeTaskPool`. It spawns, polls with
  `bevy_tasks::futures::check_ready`, and cancels by dropping. Read
  `bevy_tasks-0.20.0` for `Task` and `check_ready` rather than trusting recalled
  examples.
- `WorldGenerator` and `Shapes` become cheap to hand to a worker: the generator
  is cloned into the Job (it is a handful of floats), `Shapes` becomes
  `Shapes(Arc<ShapeTable>)` with `Deref` to `ShapeTable`, so every existing
  `&shapes` still coerces.
- `StreamingBudget` gets its final shape: an in-flight cap per stage and a
  landing cap per stage per tick, with placeholder defaults. Ticket 06 sizes
  them; this one only has to make them mean what the spec says.
- `generate` dispatches missing Chunks inside the load extent, nearest first, up
  to the in-flight cap; Landing inserts results that are still inside `evict`
  and not already stored, drops the rest, and does the `unlit` and `dirty`
  bookkeeping `generate` does today. `retire` cancels Jobs outside `evict`.
- The tick's order is `retire`, land, dispatch.
- The test harness gains `settle()`: tick until no Job is in flight and nothing
  was dispatched or landed, with an iteration ceiling that fails the test. It
  initialises the pool once per process at a fixed thread count. Rewrite the
  tests per spec decision 9 as far as generation goes; the rest keep the old
  synchronous `relight` and `publish` and are rewritten by tickets 04 and 05.
- A compile-time assertion that `Chunk`, `ChunkLight`, `ShapeTable` and
  `Worldgen` are `Send + Sync` and `'static`, so a future non-`Send` field fails
  here and not at a distant spawn site.
- Measure the cost of a spawn and a landing for an early-out Chunk. If it is
  more than a small multiple of the early-out itself, add the inline early-out
  to generation, in one function that is easy to delete (spec "What this assumes
  about per-Chunk cost", point 4).

## Acceptance criteria

- Generation never runs on the tick: the tick's own function does not call
  `Worldgen::chunk`.
- A Job in flight for a coord the anchor then leaves lands nothing, and a test
  teleports the anchor to prove it.
- `in_flight_never_exceeds_the_cap_and_landings_never_exceed_theirs` replaces
  `the_budget_caps_work_per_tick`, asserted after every tick.
- The other tests that assumed one tick fills the volume settle first and then
  assert what they asserted before.
- Nothing outside `crate::server` can construct, poll or land a Job.
- `cargo tree` for `tesserae-core` prints one line.
- `./scripts/cargo-ci` passes.

## Comments

**Spawn cost, measured** (3 workers, 2000 jobs, `Stage::spawn` through to
`take`, Apple M5): a job that takes the early-out costs the tick 0.7-0.8 us to
spawn and 0.02 us to land, against the early-out's own 3.6 ns. About 200x the
work it replaces, and 38 us a tick at the placeholder cap of 48. So the inline
early-out the ticket allowed for is **not added**: nothing here is worth the
branch, and caves would have made it dead weight. A surface-crossing Chunk
spawned and finished at 36 us a job wall time across three workers, which is the
98 us generation cost divided by three.

What landed:

- `server/jobs.rs`: `Stage<T>` (running and finished, both counted in flight;
  spawn, non-blocking poll, take, retain, per-tick counters) and `Jobs` with a
  stage each for generate, relight and mesh. The latter two are empty until
  tickets 04 and 05. The `Send + Sync + 'static` assertion on `Chunk`,
  `ChunkLight`, `ChunkMesh`, `ShapeTable` and `Worldgen` is a `const` there.
- `StreamingBudget` is `{ in_flight, landed_per_tick }`, each a `StageCounts`.
  Relight and mesh still run on the tick and use `landed_per_tick` as their work
  per tick; the placeholder defaults keep today's 12 and 12. The doc comment is
  a TODO pointing at ticket 06.
- The tick is `reset_counts`, `poll`, `retire`, land, dispatch, then the two
  synchronous stages. A finished result lands only if its coord is inside
  `evict` of the current anchor and not stored; `retire` drops jobs outside it.
- `Shapes` is `Arc<ShapeTable>`.
- The harness has `settle()` (tick until quiet, with a ceiling) on a two-thread
  pool initialised once per test process.

Two things the tests found:

- **"Quiet" must look only inside the load extent.** A Chunk generated while the
  anchor was elsewhere can sit in `unlit` for good once the anchor returns,
  because `relight` only scans the load volume. Requiring `unlit` to be empty
  made one test (and, in `dev`, the `settled` figure) never go true after a
  move. Both now use `MeshState::unlit_within`.
- `the_budget_caps_work_per_tick` is replaced by
  `in_flight_never_exceeds_the_cap_and_landings_never_exceed_theirs`. The first
  tick's dispatch count is exactly the cap whatever the workers do, which is the
  deterministic anchor the old count gave.
