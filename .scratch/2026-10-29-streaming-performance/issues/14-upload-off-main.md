# 14: Upload meshes off the main thread

Status: ready-for-agent
Blocked by: 01

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-upload`.

## What to build

`client/chunk_mesh.rs` clones every mesh attribute on the main thread when a
mesh lands. Build the Bevy `Mesh` off the main thread, on a task, so the main
thread only inserts the finished asset and spawns or updates the entity. The
Client still receives meshes and nothing else (ADR-0002).

01's checkpoint may show this is the sprint stall; if so, it comes first.

## Acceptance criteria

- [ ] Main-thread upload time in both sprints passes the spec's effect gate.
- [x] A mesh never lands after a newer one for the same Chunk: an ordering
      test.
- [ ] Zero rule-hole frames still; 02's despawn-with-mask-update rule holds.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.

## Comments

### 2026-10-31: built; the effect gate is pending the final review series

Branch `tech/streaming-upload`. Main-thread upload in both sprints and the
rule-hole frame count wait for the orchestrator's final series; Status stays.

- `receive_meshes` takes each `ChunkMeshReady` and `LodMeshReady` out of its
  `Messages` (drained, not read: the Client is their only reader, so the mesh
  moves into its build uncopied) and spawns the build on the compute pool:
  `chunk_mesh_to_mesh` per layer, and `BlendOrder::new`. Not the async pool:
  the Server's jobs fill it, and a build queued behind them lands frames late.
- `land_meshes` only inserts the finished `Mesh` and spawns or updates the
  entity. `deliver`, which sets the mask bit, moved from arrival to landing:
  a mesh still building never discards the coarser level drawing its region,
  and a retained Chunk is released only when the finer Chunks covering it land.
- `client/in_flight.rs` keeps each Chunk's builds oldest first. Landing takes
  the newest finished one and drops every older one, finished or not, so a
  mesh never lands after a newer one, and a Chunk re-sent every frame still
  lands whichever finishes first.
- `drop_meshes` runs before `land_meshes` and cancels the Chunk's builds: a
  mesh still building when its Chunk unloads never lands.
- The attribute layout is untouched (`chunk_mesh_to_mesh` is called as before,
  inside the task), so 08 merges on top.
- Tests: the ordering test in `in_flight`; with builds held by the jobs
  `Gate` (now `pub(crate)`, re-exported test-only from `server`), no mask bit
  while in flight and entity and bit in one frame, a retained coarse Chunk
  drawn until its children land, and an unload cancelling a build. The other
  `chunk_view` tests update until every build lands.
- `MainThreadCost::uploaded` now times the receive (drain and spawn) as well
  as the landing.

Cost: a mesh, an edit's included, usually lands a frame after it arrives
rather than in the same frame.
