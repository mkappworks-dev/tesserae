# 01: Baseline: agreement and budgets on main

Status: ready-for-agent
Blocked by: none

Spec: [LOD features](../spec.md).

## What to build

Measure what the coarse levels get wrong, and what they cost, before anything
changes.

- **Agreement test.** On Terra's default seed, over windows holding coast and
  forest, for each coarse level: the share of coarse columns whose water or
  land, and whose top Material, differ from level 0's at the column's centre in
  metres, and whether level 0 stands trees there. It records today's rates and
  asserts nothing yet; 02 makes it assert.
- **Budget series on `origin/main`**, run once in a quiet window after the
  other tickets stop building (it may land after them, but before 09),, at Terra's declared 160 m and 3000 m, with the packed-quads method (spec,
  Testing Decisions): per level, Chunks held, non-empty and meshed, quads and
  vertex bytes; total coarse vertex memory; time for each level and the whole
  field to fill from spawn; frame timing and a `--instruments gpu` series at
  `stationary` and `sprint-flight`. No shadow comparison (spec, Testing
  Decisions).

## Acceptance criteria

- [ ] The agreement test runs in CI and prints per-level rates; the rates are in
      Comments.
- [ ] Budget numbers in Comments, with the load average and the
      commit measured.
- [ ] Captures from fixed Eyes at 0.5, 1, 2 and 4 km on Terra, in one `/tmp`
      folder, the "before" of the first review batch.

## Comments

2026-11-22: Agreement test landed at `a8ad777a`; budget series and captures
pending a quiet window before 09.

The test is
`server::lod::agreement::coarse_ground_agrees_with_level_zero_at_each_column_centre_over_terra_coast_and_forest`
in the `tesserae` binary's tests, so CI's Test job runs it (it loads the built
Terra, as the other `terra_loaded` tests do). 12.6 s on a quiet host, 53 s
under a release build. Read the rates with `--nocapture`.

Method. Terra as loaded, default seed `9103431458751512577`, Terra's declared
160 m / 3000 m (six levels). Two 256 m windows (8 x 8 level-0 Chunk columns),
the nearest to spawn on a 64 m lattice that hold a coast, and a dry, wooded
point with dry, wooded neighbours 64 m out: coast at Chunk columns
`[43, -31]`, forest at `[47, -27]` (they overlap by 4 x 4). Every coarse column
whose centre lies in the window, against the level-0 column under that
centre. Level 0's ground is the bare generator level 1 downsamples (no trees,
Structures or Settlements), so it is the ground beneath any cover; "level 0
stands a tree" reads the full World for a trunk or canopy Voxel in that
column. Wet is the lake's Material on top; top Material is the first
non-air Voxel under any water. Coastal columns are those whose level-0 centre
is wet unlike a neighbouring coarse column's; "coastal wet, coarse dry" is a
share of all coastal columns. The coarse Chunk is what
`dispatch` makes for an untouched coord (downsample to level 2, generated at
its own size from 3); the mesh job's lake levelling is not applied.

| Coast  | columns | level 0 wet | wet/dry differs | coastal | coastal wet, coarse dry | top Material differs | level 0 stands a tree |
| ------ | ------- | ----------- | --------------- | ------- | ----------------------- | -------------------- | --------------------- |
| 1      | 16384   | 63.2%       | 0.0%            | 256     | 0.4%                    | 2.0%                 | 8.4%                  |
| 2      | 4096    | 63.1%       | 1.8%            | 128     | 49.2%                   | 18.7%                | 8.6%                  |
| 3      | 1024    | 62.9%       | 3.4%            | 64      | 48.4%                   | 3.3%                 | 9.6%                  |
| 4      | 256     | 63.3%       | 26.6%           | 32      | 50.0%                   | 3.9%                 | 8.6%                  |
| 5      | 64      | 64.1%       | 7.8%            | 16      | 25.0%                   | 4.7%                 | 7.8%                  |

| Forest | columns | level 0 wet | wet/dry differs | coastal | coastal wet, coarse dry | top Material differs | level 0 stands a tree |
| ------ | ------- | ----------- | --------------- | ------- | ----------------------- | -------------------- | --------------------- |
| 1      | 16384   | 15.6%       | 0.1%            | 256     | 7.4%                    | 3.8%                 | 22.9%                 |
| 2      | 4096    | 15.6%       | 2.6%            | 128     | 50.0%                   | 10.7%                | 22.7%                 |
| 3      | 1024    | 15.6%       | 5.5%            | 64      | 50.0%                   | 9.6%                 | 24.4%                 |
| 4      | 256     | 15.6%       | 15.2%           | 31      | 48.4%                   | 17.2%                | 23.0%                 |
| 5      | 64      | 14.1%       | 7.8%            | 14      | 35.7%                   | 35.9%                | 21.9%                 |

- From level 2 up, about half the coastal columns are wet at level 0, and
  nearly all of those are dry at the coarse level: the coast moves one coarse
  column seaward. Level 1 holds the coast (0.4%).
- Level 4 flips 26.6% of the coast window, where 63% is wet at level 0: the
  sea level its direct generator reads, not the lake's tie (`next_up` is
  overwritten, `worldgen.rs` 1166-1169), likely drains it.
- No coarse level stands a tree, so the tree column is what every level misses:
  8-10% of the coast window and 22-24% of the forest one.
- Level 2's top Material differs on 19% of the coast window against 2-5% at
  the other levels; level 5's on 36% of the forest.
- The rates here are over each window; the spec's 10/39/40/35/31% are over six
  512 m windows and a different coastal rule, so they are not this test's
  numbers.

Provisional numbers, kept only as a check for the real series (not a
baseline). `a4e651d0`'s release `profiling` build, whose non-test code is
`origin/main` (`65037cdd`); Terra from `scripts/build-guests` at that commit;
`scripts/profile-frames`, offscreen 1920x1080, three repeats, median
`[min-max]` of the three logging-on runs. The flight and stationary fixtures
(the plan now uses `stationary` and `sprint-flight`). Host loaded: 1-minute
load 4.2 to 8.6, my own builds before, other tickets' builds starting during
the GPU run.

- Timing series (00:26-00:33): frame p50 / p95 ms, flight 3.03 [2.89-3.12] /
  5.65 [5.50-5.92], stationary 2.62 [2.58-2.84] / 4.49 [4.46-4.87]; main CPU
  p50 0.81 / 0.70.
- One `--instruments gpu` run on main (00:33-00:40): GPU frame p50 / p95 ms,
  flight 3.51 [3.46-3.54] / 3.78, stationary 2.78 / 2.95; 62-70% of frames have
  a `frame_end`. The shadow runs that followed were refused or stopped.
- Budgets in the timing series, identical across runs within 100 bytes, as
  level: held / non-uniform / meshed / non-empty meshes / quads / vertex bytes,
  stationary: 1: 1639 / 599 / 548 / 149 / 93713 / 1.50 MB; 2: 1639 / 447 / 564
  / 152 / 50308 / 0.80 MB; 3: 1639 / 298 / 564 / 145 / 22989 / 0.37 MB; 4: 1639
  / 298 / 564 / 153 / 21866 / 0.35 MB; 5: 891 / 162 / 200 / 56 / 10545 / 0.17
  MB. Level 0 holds 1639 and meshes 567.
- Held mesh memory (`capture_lod_memory`), stationary: 10.97 MB in all, 3.77
  MB of it placeholders; level 0 5.57 MB, coarse levels 1.62 MB (1: 0.76, 2:
  0.41, 3: 0.19, 4: 0.18, 5: 0.09). Flight: 11.00 MB, coarse 1.84 MB.
- Fill from spawn (`filled_s`), stationary: level 0 1.5-1.7 s, level 5
  2.0-2.1 s, levels 1-4 together at 8.7-8.9 s, so the whole field at 8.7-8.9 s.
  Flight the same within 0.4 s.

Runs and binaries: `/tmp/lod22-baseline-w2tE` (`timing-main`, `gpu-1-main`;
`summarize.py` there prints the above).
