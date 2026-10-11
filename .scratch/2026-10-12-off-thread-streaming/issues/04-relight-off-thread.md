# Light solving off the tick

Status: ready-for-agent
Blocked by: 03

The stage with cross-Chunk dependencies, and the one the spec argues is safe
rather than assumes it. Spec decisions 3, 4 (relight), 5 and 9.

## What to build

- A relight Job: a Snapshot of the centre Chunk and light and its six neighbours
  (or the synthetic open-sky border where `above_the_surface` says so, decided
  on the tick), returning the new `ChunkLight`, whether it differs from the
  Snapshot's current light, and `[bool; 6]` for `boundary_differs` per face.
  Equality and `boundary_differs` run on the worker.
- Dispatch in the existing order (highest first), up to the in-flight cap, and
  **removing the Chunk from `unlit` at dispatch**. Landing never touches `unlit`
  on its own account, which is what lets a mark made during the flight survive
  (spec decision 5, last paragraph).
- Landing applies the result only if the Chunk is the same instance
  (`Arc::ptr_eq` on the centre Chunk) and is not poisoned. Otherwise it drops
  the result and marks the Chunk `unlit` again.
- `MeshState::edited(coord)` also poisons the Chunk and its six face neighbours.
  Dispatch clears the poison. `MeshState::is_unlit` also answers true for a
  Chunk with a relight in flight.
- Audit `builder.rs`, `lightbox.rs` and `gallery.rs`, which wait for light
  before editing, and say in Comments what each now waits on.
- Rewrite `relight`'s share of the tests per spec decision 9.

## The convergence check

The test the spec's decision 5 stands or falls on. Run the relaxation to
quiescence with every Job delayed by a coord-derived pseudo-random duration, so
landing order is scrambled, at in-flight caps of 1 and of unbounded, over a
volume that includes open sky, deep rock and the surface band. Assert:

1. every loaded Chunk's stored light equals `light_chunk` of its current
   neighbours (it is a fixed point), and
2. it equals the light the synchronous pre-change algorithm produces for the
   same volume (it is the same one). Keep the old algorithm as a test-only
   oracle rather than comparing against a hard-coded snapshot.

If either fails, the spec's reasoning is wrong; stop and amend the spec before
fixing the code.

## Acceptance criteria

- The convergence test above, passing.
- An Edit with relight Jobs outstanding around it: the stale result is dropped,
  and at quiescence the lattice equals what a synchronous run of the same Edit
  produces.
- A Chunk evicted and regenerated at the same coord while its Job runs: the old
  result does not land on the new instance.
- No relight work on the tick except Snapshot construction and Landing.
- `./scripts/cargo-ci` passes.
