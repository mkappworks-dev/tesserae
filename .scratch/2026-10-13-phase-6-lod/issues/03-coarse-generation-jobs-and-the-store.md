# Coarse generation jobs and the store

Status: ready-for-agent
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
