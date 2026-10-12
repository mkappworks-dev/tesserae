# Does the streaming budget still hold

Status: ready-for-agent
Blocked by: 01, 04

## What to build

- `TESSERAE_SMOOTHING=0` in `server::worldgen_params` (dev only), documented
  beside `TESSERAE_CAVES`.
- `scripts/frame-runs 5` with smoothing off and on, interleaved, ten runs each.
  Read `worst_tick_ms`, `worst_cost_*` and `mean_upload_us`, not frame time.
- Recompute the staleness window with the new per-Chunk costs.
- If a number moves, change `StreamingBudget` and its doc comment; if none does,
  say so there with the new measurement. The doc comment keeps no number whose
  reason is gone.
- Update DESIGN.md's budget item and the line about worldgen and shaped Voxels.

## Acceptance criteria

- Tables in Comments with machine and load; the worst tick is stated against the
  15.6 ms of a tick at 64 Hz.
- `./scripts/cargo-ci` passes.

## Comments
