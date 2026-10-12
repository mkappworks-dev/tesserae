# Bench smoothed terrain in all three stages

Status: ready-for-agent
Blocked by: 01, 02

Spec "Performance". The baseline `before` was saved from `f232c4e` before any
code (`cargo bench -p tesserae-core -- --save-baseline before`).

## What to build

- `worldgen/straddling_unsmoothed` (`smoothing: None`) beside `straddling`, so
  the generator's own cost is separable from the input change.
- `mesh_chunk/straddling_unsmoothed` and `light_chunk/*_unsmoothed` rows only
  where they separate input from code.
- A bench for a Chunk at the generator's budget (the synthetic ramp), because
  that is the worst case a default World can reach.
- The existing benches keep their names. The caveless `worldgen()` helper now
  returns smoothed terrain on purpose (that is what the task asks to re-bench);
  the cave benches pin `smoothing: None` so they stay like for like.
- Compare to the saved baseline: `cargo bench -p tesserae-core -- --baseline
  before`, then re-run suspects in isolation (a full run is disturbed; the caves
  PR saw 349 us read as 682 us).

## Acceptance criteria

- Before/after table in Comments with machine and conditions, naming which rows
  changed input and which changed code, including `worldgen/straddling`.
- Any regression beyond noise is explained or reworked.
- `cargo bench --workspace --no-run` builds without `--features dev`.

## Comments
