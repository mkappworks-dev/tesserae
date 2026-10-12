# Light solving off the tick

Status: done
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

## Comments

The convergence argument (spec decision 5) held; nothing needed amending.

What landed:

- `jobs.rs` holds the job body: `solve` takes the centre `Arc<Chunk>`, the light
  it had, and six `Border`s (`Absent`, `OpenSky`, `Loaded`), and returns a
  `Solved` with the new light, whether it changed, and the per-face plane
  comparison. Equality and `boundary_differs` run on the worker. The open-sky
  decision (`above_the_surface`) is made on the tick at dispatch, as before.
- `MeshState` gains `relighting` and `poisoned`. Dispatch moves a Chunk from
  `unlit` to `relighting`; landing never touches `unlit`. `is_unlit` is true for
  both, which is what the dev builders and `apply_pending` wait on, so none of
  them needed changing beyond a comment.
- `edited(coord)` poisons that Chunk and its six face neighbours when they have
  a solve running. `set_voxel` already calls it for the edited Chunk, every
  Chunk the incremental relight wrote, and the boundary neighbour, so the set is
  exactly the Chunks whose solve read something the Edit changed.
- Landing drops a result if poisoned or if `Arc::ptr_eq` against the store's
  current Chunk fails, counts it with `Stage::discard`, and marks the Chunk
  `unlit` again.

Tests, each mutation-checked by breaking the thing it guards and watching it
fail:

- `relight_in_scrambled_landing_order_reaches_the_synchronous_fixed_point`: caps
  of 1, 4 and unbounded, each job delayed 0-400 us by its coord, against an
  oracle that sweeps every Chunk until a pass changes nothing. Not marking
  neighbours at all fails it.
- `an_edit_while_a_relight_is_in_flight_drops_that_result`: fails with the
  poisoning removed.
- `a_solve_of_a_chunk_that_was_replaced_meanwhile_does_not_land`: fails with the
  `ptr_eq` check removed.
- `a_mark_made_while_a_solve_runs_survives_its_landing`: fails if landing clears
  `unlit`. The scrambled test does **not** catch that mutation: a lost mark only
  matters when the lost re-solve would have changed the answer, which random
  ordering rarely arranges. The explicit test holds one job and marks the Chunk
  mid-flight.

Test hooks live on `Stage` and are `cfg(test)`: `hold` keeps jobs from starting
their work, `scramble` delays each by a coord-derived time.

`ticket 01` guessed the 12-32 ms frames at 5.3-5.9 s were the relight backlog.
Not yet re-measured; the figure to look at is `worst_fill_ms` in ticket 06.
