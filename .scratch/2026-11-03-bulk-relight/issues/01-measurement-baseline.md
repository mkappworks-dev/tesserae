# 01: Measurement harnesses and baseline

Status: ready-for-agent
Blocked by: none

Spec: [bulk relight](../spec.md).

## What to build

Harnesses that measure the costs this branch changes, run against the code as it
is, before anything else changes.

- `crates/tesserae-core/benches/relight.rs` (registered in the crate's
  `Cargo.toml`): a lattice of Chunks with open sky above and a stone floor, and
  a 16³ stone box placed into the air over it, one `relight` per Voxel. A
  second case removes the same box. A third places a 16×1×16 roof of stone, the
  worst case for sky darkening. Throughput is reported in Voxels (criterion
  `Throughput::Elements`).
- An `#[ignore]`d test in `server/mods/` that loads lumen at 0.5 m through the
  demo-Games install helper, streams and lights the World around the build with
  the streaming test `Harness` at the default `StreamingBudget`, runs the Game's
  `tick` until its build batches are queued, then alternates the real
  `PendingCommands::drain` with a streaming tick until the queue is empty. It
  prints ticks to drain, seconds at 64 Hz, drain CPU per tick (mean and worst,
  thread CPU time), and Voxels a second of drain CPU. An env var overrides the
  budget so a sweep needs no rebuild.
- The same test, or a sibling, drains a plain 16³ stone fill Command over open
  air, for "Voxels a second for a plain fill" at the Server level.

## Acceptance criteria

- [ ] `cargo bench -p tesserae-core --bench relight` runs and reports Voxels a
      second for each case.
- [ ] The ignored lumen test prints the figures above at the current budget of
      2, and finishes (it may take minutes at 2 a tick).
- [ ] The baseline figures, host load and command lines are in Comments.

## Comments
