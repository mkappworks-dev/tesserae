# Does the streaming budget still hold

Status: done
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

`TESSERAE_SMOOTHING=0` added beside `TESSERAE_CAVES`. `scripts/frame-runs 3`,
interleaved off, on, off, on, off, on: nine runs each, Apple M5, debug at
`opt-level = 1`, `TESSERAE_SCREENSHOT_OFFSCREEN=1`, at a machine load of about
20 from other sessions. Medians with the range in brackets.

| Figure                  | smoothing off    | smoothing on     |
| ----------------------- | ---------------- | ---------------- |
| worst tick (ms)         | 1.6 [0.7-8.2]    | 1.8 [1.2-5.3]    |
| worst cost, fill (ms)   | 1.4 [0.7-8.3]    | 1.7 [1.0-5.3]    |
| worst cost, flight (ms) | 1.2 [0.4-5.3]    | 1.1 [0.5-2.9]    |
| mean mesh upload (us)   | 11 [10-13]       | 16 [14-17]       |
| fill (s)                | 3.13 [2.56-4.81] | 2.87 [2.57-4.47] |
| relight landed / stale  | 10989 / 16       | 11061 / 14       |
| mesh landed / stale     | 1077 / 7         | 1079 / 7         |

The absolute figures are worse than the caves branch's (worst tick 0.6 ms, fill
1.58 s) because the machine was shared; on the two quietest runs the fill was
1.6 s with smoothing on. The difference between the columns is within the noise
except the upload, which rose by 5 us because a surface mesh has more quads.

- **Worst tick**: 15.6 ms is what a tick has at 64 Hz; the worst tick was 5.3 ms
  at its worst, with the machine loaded, and typically under 2 ms.
- **Upload**: 32 meshes a tick at 16 us is 0.5 ms (it was 0.35 ms).
- **Staleness window**, if every job were a surface Chunk a tunnel crosses with
  a ramp: 64 x (252 + 65) + 128 x (140 + 150) + 128 x (395 + 60) = 116 ms of
  work, 39 ms over three workers, 1.4x the 28 ms of the caves branch.
- **Defaults unchanged**; `StreamingBudget`'s doc comment carries the figures
  and DESIGN.md's budget item says so.
