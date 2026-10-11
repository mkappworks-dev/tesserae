# The Jobs resource, and generation off the tick

Status: ready-for-agent
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
