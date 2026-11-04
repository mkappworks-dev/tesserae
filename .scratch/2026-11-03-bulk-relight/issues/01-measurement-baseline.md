# 01: Measurement harnesses and baseline

Status: done
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

- [x] `cargo bench -p tesserae-core --bench relight` runs and reports Voxels a
      second for each case.
- [x] The ignored lumen test prints the figures above at the current budget of
      2, and finishes (it may take minutes at 2 a tick).
- [x] The baseline figures, host load and command lines are in Comments.

## Comments

### Baseline, 2026-11-04 (base `aa96135`, Apple M5, load 17-29)

Server harness, test profile, thread CPU of the drain call only:

```
cargo test -p tesserae --all-features --bin tesserae -- --ignored --exact \
  server::mods::runtime::tests::measure_lumen_build_drain --nocapture
TESSERAE_DRAIN_BUDGET=<n> ... measure_plain_fill_drain
```

| Build           | Budget | Ticks | s at 64 Hz | Mean drain ms | Worst drain ms | Voxels / drain CPU s |
| --------------- | ------ | ----- | ---------- | ------------- | -------------- | -------------------- |
| lumen 19,712    | 2      | 9,856 | 154.00     | 0.045         | 0.64           | 44,433               |
| lumen           | 32     | 616   | 9.62       | 0.74          | 7.63           | 43,414               |
| lumen           | 256    | 77    | 1.20       | 6.48          | 47.97          | 39,492               |
| lumen           | 1024   | 20    | 0.31       | 22.56         | 109.54         | 43,686               |
| plain fill 4096 | 2      | 2,048 | 32.00      | 0.015         | 0.46           | 135,845              |
| plain fill      | 32     | 128   | 2.00       | 0.17          | 3.33           | 186,180              |
| plain fill      | 256    | 16    | 0.25       | 1.17          | 12.78          | 219,190              |
| plain fill      | 1024   | 4     | 0.06       | 5.83          | 19.24          | 175,676              |

The cost a Voxel is flat with the budget (about 23 us for lumen, 5-7 us for
the fill), as one relight per Voxel predicts. Lumen meets 10 s at 32 a tick,
but its worst drain tick there is 7.6 ms, four times the 2 ms ceiling: a few
Voxels (roof over sky) cost far more than the mean.

Core bench, release, criterion wall time (median, 10 samples):

| Case                      | Time     | Voxels a second |
| ------------------------- | -------- | --------------- |
| place_box_16 (4096)       | 5.67 ms  | 723 K           |
| remove_box_16 (4096)      | 9.83 ms  | 417 K           |
| roof_16x16 (256 over sky) | 18.48 ms | 13.9 K          |
