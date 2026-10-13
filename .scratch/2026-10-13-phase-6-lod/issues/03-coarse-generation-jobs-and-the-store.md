# Coarse generation jobs and the store

Status: done
Blocked by: 01, 02

Spec decisions 1 and 5 (generation half). Read ADR-0007 first.

## What to build

- `server/lod.rs`: Server-private `LodStore` (per level
  `HashMap<ChunkCoord, Arc<Chunk>>`) and `LodState` (per level `dirty`, `stale`,
  `delivered`), `pub(in crate::server)` like `ChunkStore`. Registered from the
  streaming plugin; `stream_lod` runs in `FixedUpdate` after `stream`.
- `Stage<T>` in `jobs.rs` gains a key parameter defaulting to `ChunkCoord`; the
  coarse stages are keyed by `(level, ChunkCoord)`. Level-0 stages and their
  tests are untouched (the test-only `scramble_yields` hashes through `Hash`).
- Source generators: one `Worldgen` per source level `j` at `voxel_size * 2^j`,
  built in `open_world`. `j = 0` is the production generator; `j >= 1` has
  `smoothing: None` (a comment says why: the smoothing gate is by metres and a 2
  m Voxel passes it).
- Generate job for level `k >= 1`: clones the generator of level `k-1`; for
  `k == 1` it takes the `Arc<Chunk>` of each held level-0 child from the
  `ChunkStore` at dispatch and generates the rest; calls `downsample`; returns
  an owned `Chunk`. No stage function has a store in its signature.
- Landing: lands if inside `evict` of the level's anchor and absent or `stale`,
  else dropped. Dispatch removes the coord from `stale`. Landing a replacement
  marks held face neighbours `dirty`.
- Dispatch: after level 0, nearest level first, nearest Chunk first; caps shared
  across levels in a new `LodBudget` (16 generate in flight, 16 landed per tick
  as placeholders). `retire`/scan only when the level's anchor Chunk changed, a
  job of the level landed, or an invalidation happened; the offset lists are
  shared between levels with the same extent.
- Under `dev`, `TESSERAE_LOD=0` turns coarse streaming off.
- With one level (`lod_distance_m <= view_distance_m`) nothing here runs.

## Tests

(Settle-style: pump ticks to quiescence with an iteration ceiling.)

- At quiescence every held coarse Chunk equals `downsample` of its eight
  generated children, for levels 1, 2 and 3.
- Held level-0 children are used when present and the result is identical to
  generating them (the property decision 1 stands on: the generator reproduces
  an unedited Chunk exactly).
- Caps: in flight never exceeds the cap, landings never exceed theirs, sampled
  every tick.
- Moving the anchor loads ahead and evicts behind, per level, and does not
  thrash on a Cell boundary.
- A job in flight for a Chunk the anchor then leaves lands nothing.
- With `lod_distance_m == view_distance_m` no coarse Job is ever spawned.
- Scrambled landing order reaches the same quiescent state.

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- Level-0 streaming tests pass unchanged.

## Comments

Built as specified except where listed. `server/lod.rs` holds `LodStore`,
`LodState`, `LodJobs`, `LodBudget`, `LodGenerators`, the job
(`generate_coarse`) and the tick (`stream_lod`, chained after `stream` in
`streaming::plugin`). 17 tests in `lod.rs`; the rest of the suite is untouched.
Mutation checks, each restored: dropping the `evict` test at landing fails
`a_result_outside_evict_is_dropped_at_landing`; dropping the "held and not asked
for" test fails `a_result_for_a_held_chunk_nothing_asked_to_replace_is_dropped`;
mapping a held child to the wrong octant at dispatch fails three tests (the
octant test, the invalidation test and the held-children-equal-generated test).

Where the spec was wrong or silent:

- **Landing "absent or `stale`" cannot land a regeneration.** Dispatch removes
  the coord from `stale`, so a replacement finishing later finds the Chunk held
  and not stale and is dropped. `Level` has a `regenerating` set, filled at
  dispatch for a held coord and consumed at landing, as `MeshState::relighting`
  is for light. Landing is: inside `evict`, and absent or regenerating. A mark
  made during the flight is still in `stale`, so a fresh job follows.
- **The sources are built by a Startup system after `open_world`**
  (`open_lod_generators`), not inside it. It reads the production params back
  from `Worldgen::params()`, so the `TESSERAE_CAVES`/`TESSERAE_SMOOTHING`
  toggles reach every level, and `open_world` and its tests keep their
  signature. `LodGenerators::sources[j]` makes level j, so a level-k job clones
  `sources[k - 1]`; there is one fewer source than levels.
- **`TESSERAE_LOD=0` sets `lod_distance_m = view_distance_m`** in
  `StreamingSettings` (dev only) rather than adding a flag. One level is already
  "nothing runs", and the Client will see one level too.
- **The "scan only when something happened" gate** is one `pending` flag per
  level. Set by an anchor move, a landing (used or dropped), an invalidation,
  and by a scan the budget cut short; a level after the one that spent the
  budget keeps its flag and is scanned on a later tick. Without the last rule a
  level starved by a nearer one would wait for its own anchor to move.
  `retire` runs only on an anchor move, which is the only thing that can push a
  Chunk out of `evict`.
- `LodCounts` has only `generate`; ticket 04 adds `mesh` (32 / 16 in the spec)
  beside it, and ticket 07 replaces the 16 / 16 placeholders.
- `Stage<T, K = ChunkCoord>`: `K: Copy + Eq + Hash + Debug`. `scramble_yields`
  now hashes through `DefaultHasher`, so the level-0 scramble order changed
  (it is still unrelated to dispatch order, which is all the tests need).
- `LodStore` and `LodState` are indexed `level - 1` and grow on first write, so
  neither needs the level count. Level 0 is a `debug_assert`.

Test harness: the existing `streaming::tests::Harness` is reused. Its module,
the struct, `app`, `anchor`, `new`, `tick`, `anchor_at`, `store`, `settle`,
`small`, `UNLIMITED` and `SETTLE_CEILING` became `pub(in crate::server)`, and
`Harness::new` registers the lod resources; no level-0 test or assertion
changed. `lod.rs` adds `lod_tick`/`lod_settle` to it in an `impl Harness`.
`lod_settle` stops at 200,000 ticks as well as the 60 s ceiling, so a livelock
that never waits on a job fails quickly. The tests use four levels (reach 256 m
over the `small()` settings), where every level is mesh (1, 1), load (3, 3),
evict (4, 4): about 200 Chunks a level.

For ticket 04:

- `dirty` is set at landing: the Chunk itself, and on a replacement its held
  face neighbours. First generation marks only itself. `delivered` exists and is
  pruned on eviction, but nothing inserts into it: `retire` must write
  `LodUnloaded` for a delivered Chunk before it is pruned, and today it is
  pruned silently.
- `LodStore::shared`, `get` and `coords` carry a one-line
  `cfg_attr(not(test), allow(dead_code))` each, as does
  `LodState::invalidate` (ticket 05 calls it). Remove them as callers arrive.
- `LodJobs` is separate from `Jobs`; add the mesh `Stage<ChunkMesh, LodKey>`
  there, and `LodCounts::mesh`. `Stage::keys` is test-only.
- Dispatch scans a level's `load` offsets, shared by extent via
  `LodState::offsets_for`; a mesh scan wants the shell's offsets
  (`Shell::offsets`) and should be gated the same way, with its own `pending`.
- `stream_lod` returns early with one level and when no Player exists. In the
  dev build `cost::tick_ended` now runs after `stream_lod`, so `MainThreadCost`
  includes coarse streaming.
- `streaming::{face_neighbors, offset, delta}` are `pub(super)` for `lod.rs`.
