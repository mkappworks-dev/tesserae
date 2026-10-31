# Release a World session's wgpu buffers and bind groups

Status: done
Ticket: [34](../2026-10-27-gameplay-performance-and-controls/issues/34-world-session-memory-growth.md),
the optional bounded PR.

## Problem

Each World opened and left in one process leaves about 60 wgpu buffers and
120 bind groups alive. Between soak cycles 3 and 25, `malloc_history` counted
live `create_buffer` blocks 526 to 2011 and `create_bind_group` 221+217 to
1292+1180; `heap` counted `AGXG17GFamilyBuffer` 454 to 1821. Entities,
resources, meshes and images are flat at every return, so the objects are
held below the ECS: in a Bevy render-world cache or allocator, or in wgpu's
deferred destruction. The Rust live heap rises 0.07 MB a cycle, and these
objects may account for most of it (unconfirmed).

## What to find

The holder, by evidence rather than by reading:

1. Count. wgpu's internal counters (`Device::get_internal_counters`, behind
   wgpu's `counters` feature) give live buffers and bind groups per return;
   `malloc_history` with deeper stacks names the Bevy caller of each
   surviving `create_buffer` and `create_bind_group`.
2. Name the owner from the callers: a render-world resource keyed by view or
   material that outlives the session's cameras and materials, an allocator
   slab never freed, or a resource wgpu keeps until a `poll` that never
   comes.
3. Release it at session teardown, at the narrowest seam that owns it.

## Decisions

- The fix belongs to session teardown (`session.rs`, client teardown) or to
  the render-world code that owns the holder. Job and Chunk teardown are out
  of bounds: `tech/streaming-performance` edits `server/jobs.rs`,
  `streaming.rs` and `client/chunk_view.rs`.
- If the holder is inside Bevy and not reachable from our code, record it
  with the upstream source line and stop; no fork or patch of Bevy.
- Instrumentation is temporary: it lives in
  `issue-34/instrumentation.patch` and is reverted before the branch ends.

## Acceptance

- Live wgpu buffer and bind group counts equal at cycle 3 and cycle 25 of a
  release soak, by the counters or the `malloc_history` diff.
- Rust live bytes flat across cycles (within noise of the counting
  allocator), or the remaining source named.
- Entities, resources, meshes and images still flat; existing tests pass
  under `scripts/cargo-ci`.
- Ticket 34 updated with the holder, the fix and the numbers, hand-wrapped at
  80 columns.

## Method

`issue-34/soak.sh` with the demo rebuilt from this tree first (a stale
`demo.wasm` makes every open fail at `declare`), release with
`tesserae/dev`, offscreen, `MallocStackLogging=1` and `T34_HEAP=3,25`. Cargo
under `caffeinate -i` with `RUSTFLAGS="-D warnings"`. No push or PR without
asking.
