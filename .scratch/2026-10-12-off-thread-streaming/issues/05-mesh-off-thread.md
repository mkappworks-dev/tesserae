# Meshing off the tick

Status: ready-for-agent
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
