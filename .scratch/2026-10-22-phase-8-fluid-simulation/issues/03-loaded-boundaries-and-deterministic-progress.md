# 03: Carry flow across loaded Chunks deterministically

Status: ready-for-agent
Blocked by: 02

Spec: [decisions 6 and 7](../spec.md).

## What to build

Make streams cross loaded Chunk boundaries, pause at absent terrain, and
resume on admission, with deterministic logical progress and fair bounded
work during concurrent player and Mod Edits.

## Acceptance criteria

- [ ] Tests cross all six faces, negative coordinates, and different Chunk
  admission orders. Missing terrain is unknown, never Air.
- [ ] Admission wakes both sides; eviction drops local state/work and wakes
  held dependents. A new Chunk at an old coordinate rejects old-lifetime Jobs.
- [ ] No fluid-driven terrain loading, offline catch-up, or persistence is
  added. Generated sources return on regeneration under existing semantics.
- [ ] Logical steps define when ordered Edits, Mod Commands, admission, and
  eviction enter the simulation. Worker completion order is not that order.
- [ ] Equal step-indexed events produce equal canonical simulation state
  under different hash seeds, worker counts, quotas, and scrambled completions.
- [ ] Live Edits invalidate affected proposals without globally restarting
  unrelated work, starving player Edits, or landing partial transactions.
- [ ] Evaluation, activation scans, wakeups, retained results, in-flight Jobs,
  and landing are count-bounded. Stale/rejected/blocked attempts count as work.
- [ ] Queue saturation preserves continuation or explicit backpressure.
  Coalesced per-Chunk activation work is resumable; nothing required is dropped.
- [ ] Player Edits, fair Mod queues, and each active Chunk make progress under
  saturation. The two-Voxel Mod drain cap stays unchanged.
- [ ] Quiescent fluid and cold unburning terrain perform no recurring
  per-Voxel simulation evaluation.

## Verification and completion

Extend held/scrambled Job tests and the controlled Server-tick harness.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Determinism controls the authoritative logical event trace. Equal wall-clock
  ticks with differently admitted terrain or Mod batches are not equal inputs.
