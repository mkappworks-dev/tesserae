# Bench smoothed terrain in all three stages

Status: done
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

Apple M5, release, one binary per side (`main` at `f232c4e` built in its own
worktree), run alternately three times, minimum of three, load average 7 to 20
from other sessions. Rows that did not change code differ by up to 10% between
sides, so read less than that as noise.

| Bench                                  | main     | branch   | Reads as             |
| -------------------------------------- | -------- | -------- | -------------------- |
| `worldgen/straddling`                  | 111.6 us | 178.1 us | code and input, +60% |
| **`worldgen/straddling_unsmoothed`**   | -        | 112.6 us | the same as main     |
| **`worldgen/ramp_densest`**            | -        | 178.9 us |                      |
| `worldgen/early_out_air` / `_stone`    | 4 ns     | 4-5 ns   | unchanged            |
| `mesh_chunk/straddling`                | 380.8 us | 410.9 us | input, +8%           |
| **`mesh_chunk/straddling_unsmoothed`** | -        | 346.6 us |                      |
| **`mesh_chunk/ramp_densest`**          | -        | 457.3 us | the worst real Chunk |
| `mesh_chunk/declined_*`                | 6-7 ns   | 7 ns     | unchanged            |
| `light_chunk/straddling`               | 98.0 us  | 175.6 us | input, +79%          |
| `light_chunk/straddling_settled`       | 151.5 us | 301.0 us | input, +99%          |
| **`light_chunk/ramp_densest`**         | -        | 216.6 us |                      |
| `light_chunk/straddling_shaped`        | 157.0 us | 155.7 us | unchanged            |
| `light_chunk/flooded_air`              | 128.8 us | 129.1 us | unchanged            |
| `light_chunk/open_sky` / `opaque`      | 3-4 ns   | 3-4 ns   | unchanged            |
| `greedy_mesh/terrain` (`testgen`)      | 363.5 us | 341.5 us | unchanged            |
| `greedy_mesh/terrain_stairs_at_cap`    | 557.9 us | 531.9 us | unchanged            |
| **`greedy_mesh/terrain_stairs_at_budget`** | -    | 452.9 us | 512 stairs           |

Read it as:

- **Generation is +65 us on a surface Chunk**, none of it the border (a World of
  cubes reads the same as main). The pass itself, and probably the palette
  growing from four entries to five, which re-packs all 32768 indices once; that
  part was not isolated. A division-free re-pack was tried and showed nothing
  outside noise, so it was not kept.
- **Light is the real cost, and it is the step-5 shaped flood arriving**:
  `straddling_shaped` (a stair on every column, hand-built) was already 66% over
  `straddling` at step 5, and a surface Chunk with any ramp now takes that path.
  It is worker time, once per surface Chunk, and a Chunk that holds no ramp does
  not pay it. Making the shaped flood cheaper is the lever, and is not this
  change.
- **Mesh +8% typical, +20% densest**: pass 2 at about 170 ns a stair, as step 5
  measured.
- Everything the change cannot reach (the `testgen` rows, the fast paths) is
  within noise.
