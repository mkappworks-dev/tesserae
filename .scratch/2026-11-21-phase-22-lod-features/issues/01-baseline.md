# 01: Baseline: agreement and budgets on main

Status: done
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

- [x] The agreement test runs in CI and prints per-level rates; the rates are in
      Comments.
- [x] Budget numbers in Comments, with the load average and the
      commit measured.
- [x] Captures from fixed Eyes at 0.5, 1, 2 and 4 km on Terra, in one `/tmp`
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

2026-11-22: Budget series and captures on `main` at `65037cdd` (=
`origin/main`), in a quiet window; these replace the provisional numbers above.

Method. `main` in a temporary worktree; release `profiling` build for the
series, release `dev` build for the captures; Terra from `scripts/build-guests`
at that commit, and a second Terra built the same way with only
`view_distance_m: 192.0` (lod 3000) for the 192 m column.
`scripts/profile-frames` from that worktree, offscreen 1920 x 1080, `stationary` and `sprint-flight`,
three interleaved repeats per run. Timing in mirrored order (160, 192, 192,
160), then the `--instruments gpu` series the same way: six logging-on runs
per fixture and radius, median `[min-max]`. No shadow series. Each run started
only once the 1-minute load was under 3; the engine itself lifts it to 5 to 7
while running, and nothing else ran.

| run (14:02-15:14) | load at start (1/5/15 min) | load at end     |
| ----------------- | -------------------------- | --------------- |
| timing-160-a      | 2.92 8.20 9.82             | 5.21 6.20 8.19  |
| timing-192-a      | 2.70 5.36 7.76             | 7.04 6.02 7.14  |
| timing-192-b      | 2.68 8.78 9.02             | 5.42 6.37 7.71  |
| timing-160-b      | 2.86 5.53 7.32             | 6.44 5.92 6.83  |
| gpu-160-a         | 2.93 4.86 6.32             | 6.68 6.28 6.50  |
| gpu-192-a         | 2.83 5.01 5.96             | 6.69 6.40 6.33  |
| gpu-192-b         | 2.89 5.28 5.91             | 5.69 5.76 5.94  |
| gpu-160-b         | 2.85 4.88 5.59             | 6.53 5.86 5.79  |

Per coarse level at 160 m / 3000 m, identical in every run except level 1 in
`sprint-flight` (within 3%): Chunks held / non-uniform / meshed / non-empty
meshes / quads / vertex bytes the Server delivered (16 B a quad).

| level | stationary                                | sprint-flight                             |
| ----- | ----------------------------------------- | ----------------------------------------- |
| 1     | 1639 / 599 / 548 / 149 / 93713 / 1.50 MB  | 1892 / 688 / 660 / 150 / 60088 / 0.96 MB  |
| 2     | 1639 / 447 / 564 / 152 / 50308 / 0.80 MB  | 1892 / 516 / 672 / 170 / 74255 / 1.19 MB  |
| 3     | 1639 / 298 / 564 / 145 / 22989 / 0.37 MB  | 1892 / 344 / 672 / 165 / 29466 / 0.47 MB  |
| 4     | 1639 / 298 / 564 / 153 / 21866 / 0.35 MB  | 1804 / 328 / 644 / 176 / 27452 / 0.44 MB  |
| 5     | 891 / 162 / 200 / 56 / 10545 / 0.17 MB    | 1012 / 184 / 252 / 72 / 14559 / 0.23 MB   |
| all   | 3.19 MB delivered, 1.62 MB held           | 3.29 MB delivered, 1.68 MB held           |

Level 0 holds 1639 and meshes 567 (`sprint-flight`: 1892 / 672). Held mesh
memory (`capture_lod_memory`, the Client's own meshes), stationary: 10.97 MB
in all, 3.77 MB placeholders, level 0 5.57 MB, coarse 1.62 MB (1: 0.76, 2:
0.41, 3: 0.19, 4: 0.18, 5: 0.09). `sprint-flight` is read after the flight, so
level 0 holds 1.23 MB there.

Fill from spawn (`filled_s`), s, median `[min-max]` of six timing runs:

| fixture, radius     | level 0          | levels 1-4 (192: 1-3) | top level        | whole field         |
| ------------------- | ---------------- | --------------------- | ---------------- | ------------------- |
| stationary, 160     | 1.51 [1.31-1.63] | 8.70 [8.42-8.97]      | 5: 1.96          | 8.71 [8.44-8.97]    |
| sprint-flight, 160  | 1.56 [1.45-1.64] | 8.74 [8.66-8.86]      | 5: 1.99          | 8.74 [8.67-8.86]    |
| stationary, 192     | 1.83 [1.55-1.94] | 10.10 [9.69-10.25]    | 4: 3.37          | 10.10 [9.70-10.25]  |
| sprint-flight, 192  | 1.85 [1.80-2.00] | 10.05 [10.02-10.31]   | 4: 3.33          | 10.06 [10.02-10.31] |

Frame and GPU, ms, median `[min-max]` of six runs each:

| fixture, radius    | frame p50        | frame p95        | main CPU p50 | GPU frame p50    | GPU frame p95    |
| ------------------ | ---------------- | ---------------- | ------------ | ---------------- | ---------------- |
| stationary, 160    | 2.73 [2.52-2.96] | 4.75 [4.29-5.14] | 0.71         | 2.70 [2.67-2.86] | 2.97 [2.79-3.27] |
| sprint-flight, 160 | 3.13 [2.86-3.95] | 6.75 [6.37-7.93] | 0.77         | 3.19 [3.11-3.37] | 3.90 [3.74-4.13] |
| stationary, 192    | 2.70 [2.63-2.74] | 5.05 [4.80-5.12] | 0.71         | 2.88 [2.86-3.02] | 3.16 [2.95-3.41] |
| sprint-flight, 192 | 3.55 [3.22-3.84] | 7.33 [6.85-7.67] | 0.80         | 3.20 [3.12-3.29] | 4.05 [3.92-4.27] |

52-79% of GPU-series frames have a `frame_end` (37% in one 192 stationary run).

160 m against 192 m:

- At 192 m the 3000 m reach takes five levels, not six (`192 * 2^4` > 3000):
  level 4 is the top and fills at 3.4 s, as level 5 does at 2.0 s at 160 m.
- Level 0 holds 2167 Chunks against 1639 (+32%) and meshes 791 against 567;
  its held mesh memory is 8.06 MB against 5.57.
- Level 0 fills 0.3 s later (1.83 against 1.51 s stationary, 1.85 against
  1.56 in flight), and the whole field 1.4 s later (10.1 against 8.7 s): over
  09's 10 s budget on `main` already.
- GPU frame p50 is 0.18 ms higher at `stationary` (2.88 against 2.70) and the
  same in flight (3.20 against 3.19); frame p95 0.3 to 0.6 ms higher.
- Coarse vertex bytes delivered 3.90 MB against 3.19 stationary, 4.30 against
  3.29 in flight; held coarse 1.98 against 1.62 MB.

Also seen:

- Levels 1 to 4 all reach `filled_s` within 0.02 s of each other at 8.7 s, and
  the top level at about 2 s: the coarse field waits on one shared step, not on
  each level's own work.
- Level 1's `shown_s` is `never` at `stationary` and 20.6 s (when the flight
  starts) in `sprint-flight`, at both radii, though its 548 meshes are held:
  presumably level 0 covers its ring until the Eye moves; not investigated.

Captures (not committed): `/tmp/lod22-01-baseline-yovc/captures/`, the
"before" of the first review batch. Release `dev` build of `main`, Terra at
160 m / 3000 m, offscreen 1920 x 1080, no flight, settle 30 s, yaw 0 (looking
north at the forest window at about z -736), on Voxel planes:

| capture  | Eye (x, y, z)            | pitch |
| -------- | ------------------------ | ----- |
| at-0.5km | 1760.5, 200.5, -235.5    | -13.5 |
| at-1km   | 1760.5, 240.5, 264.5     | -9    |
| at-2km   | 1760.5, 300.5, 1264.5    | -6.5  |
| at-4km   | 1760.5, 420.5, 3264.5    | -5    |

At 4 km the view reaches past Terra's 3 km edge and shows fog. Level 0's held
mesh is empty at all four Eyes (each is about 160 m or more over the ground).
Runs, logs, the 192 m Terra and `summarize.py` (prints every table above from the run
folders) are in `/tmp/lod22-01-baseline-yovc`.

To rerun against a branch (09), one fresh output folder per run:

```sh
export RUSTFLAGS="-D warnings"
cargo build --locked -p tesserae --features profiling --release
cp target/release/tesserae "$OUT/bin/tesserae-profiling"
cargo build --locked -p tesserae --features dev --release
cp target/release/tesserae "$OUT/bin/tesserae-dev"
scripts/build-guests && cp -R games/terra/dist "$OUT/terra160/"
# 192 m: set view_distance_m: 192.0 in games/terra/src/lib.rs, rebuild
# terra.wasm into a copy of dist ($OUT/terra192/dist), then restore the file.

# Output path first: --fixtures and --instruments swallow a trailing one.
# Order: timing 160, 192, 192, 160; then the same with --instruments gpu.
scripts/profile-frames "$OUT/timing-160-a" --binary "$OUT/bin/tesserae-profiling" \
  --profile release --features profiling --game "$OUT/terra160/dist/mods" \
  --repeats 3 --fixtures stationary sprint-flight
scripts/profile-frames "$OUT/gpu-160-a" --binary "$OUT/bin/tesserae-profiling" \
  --profile release --features profiling --game "$OUT/terra160/dist/mods" \
  --repeats 3 --fixtures stationary sprint-flight --instruments gpu
python3 summarize.py timing-160-a timing-160-b

# One capture; the others change only the Eye and pitch above.
env -i HOME="$HOME" PATH=/usr/bin:/bin RUST_LOG=info BEVY_ASSET_ROOT="$PWD" \
  TESSERAE_MODS_DIR="$OUT/terra160/dist/mods" TESSERAE_TRUST_MODS=1 \
  TESSERAE_SCREENSHOT="$OUT/captures/at-2km.png" TESSERAE_SCREENSHOT_OFFSCREEN=1 \
  TESSERAE_SCREENSHOT_WIDTH=1920 TESSERAE_SCREENSHOT_HEIGHT=1080 \
  TESSERAE_SCREENSHOT_SPEED_M_S=0 TESSERAE_SCREENSHOT_FLY_S=0 \
  TESSERAE_SCREENSHOT_SETTLE_S=30 TESSERAE_SPAWN_M=1760.5,300.5,1264.5 \
  TESSERAE_SCREENSHOT_YAW_DEG=0 TESSERAE_SCREENSHOT_PITCH_DEG=-6.5 \
  "$OUT/bin/tesserae-dev"
```

The full scripts are `series.sh` and `captures.sh` in the run folder.
