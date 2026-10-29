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
- [ ] A mesh never lands after a newer one for the same Chunk: an ordering
      test.
- [ ] Zero rule-hole frames still; 02's despawn-with-mask-update rule holds.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.
