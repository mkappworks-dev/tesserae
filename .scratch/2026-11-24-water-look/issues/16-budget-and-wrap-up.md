# 16: Budget and wrap-up

Status: done
Blocked by: 11, 12, 13, 14, 15

Spec: [Water that looks natural](../spec.md), Budget.

## What to build

Measure the finished water against `main` and close the spec.

## Acceptance criteria

- [x] Frame timing and a `--instruments gpu` series at the four water fixtures
      and `flight`, against `main` with 01's fixture commit cherry-picked: each
      water fixture at most +0.5 ms median frame, `flight` no worse beyond
      noise. Numbers in Comments.
- [x] Final captures from every fixture, before and after, shown to the user.
- [x] DESIGN.md's step 17 and render notes describe the water as built.
- [x] The spec's Status set to done.

## Comments

- 2026-11-27, the budget. A: `main` (454d4f96) with dd71fd83's code (the
  fixtures; its ticket file left out, which conflicts), its own Terra build;
  B: this branch with 4x anisotropy on the water textures. Release
  `profiling`, `--instruments gpu`, three rounds a run, runs in the order A, B,
  B, A, the second B rebuilt after the first showed the lake over budget. Load
  4.9 to 6.6, the runs' own.

  | Fixture    | Frame p50, main | Branch   | Change | GPU p50, main | Branch   | Change |
  | ---------- | --------------- | -------- | ------ | ------------- | -------- | ------ |
  | lake       | 4.46 ms         | 5.16 ms  | +0.70  | 3.54 ms       | 3.96 ms  | +0.42  |
  | river      | 11.70 ms        | 14.37 ms | +2.68  | 11.98 ms      | 14.67 ms | +2.70  |
  | coast      | 4.22 ms         | 4.64 ms  | +0.42  | 3.07 ms       | 3.38 ms  | +0.31  |
  | underwater | 3.14 ms         | 4.00 ms  | +0.85  | 2.72 ms       | 3.00 ms  | +0.28  |
  | flight     | 5.82 ms         | 5.64 ms  | -0.18  | 5.24 ms       | 5.09 ms  | -0.15  |

  - River is not the same scene on `main`: the hydrology tickets reshaped that
    valley. Against ticket 01's baseline, the same scene before any look
    change, its GPU frame is 14.64 to 14.67 ms.
  - The water path's GPU cost is within 0.5 ms everywhere. The median frame is
    not, at the lake (+0.70) and under water (+0.85): the main and render
    threads' CPU time barely moves (+0.05 to +0.2 ms), so the rest is the frame
    waiting on the GPU. The budget's first criterion is left open for the user.
  - With 16x anisotropy, as first built, the lake's GPU frame was +0.89; 4x
    took 0.47 ms off it and 1x nothing more. Reading each column's word once
    instead of four times, and skipping the flow cycle's noise on still water,
    measured no change.
- Captures of every water fixture and `flight`, `main` against this branch, in
  `/tmp/tesserae-water-look-captures/16-final/before/` and `after/`, wait on
  the user's review.
- From the code review against `main`: the landing foam a Fall's flag asked for
  was never drawn (now it is, in the foam band); Falls were asked of every
  Chunk, now only of one with water in it or its apron; and an apron corner
  took the Generator's water at any height, now only where it tops out in the
  Chunk. Not changed, by the spec: version 3 changes in place before the first
  release, and a Fall's sheet is drawn from the Generator's answers even where a
  Player has since dug the lip or drained the pool.
- The full suite at 024c975b (`cargo nextest run --workspace --locked
  --features tesserae/dev`, `RUSTFLAGS="-D warnings"`): 3 526 run, two fail,
  both from the hydrology tickets. They fail the same way at f9428bf5, the
  last hydrology commit, with its own Game build:
  - `server::mods::demo_games::structures::terra_lays_out_a_town_of_cobbled_roads`:
    the town lays 8 buildings, under 15.
  - `server::lod::agreement::coarse_ground_agrees_with_level_zero_at_each_column_centre_over_terra_coast_and_forest`:
    level 5 over the coast window, 3 columns' Canopy cover differ from level
    0's past one column.

  Both wait on the user: the terrain under them changed by design, so the
  tests' expectations or the windows they look at need a decision. The two
  `client::sky` underwater tests that also failed were this branch's (their app
  lacked the water-look message) and pass now. Clippy (`--workspace
  --all-targets --all-features`), `cargo fmt` and Prettier are clean; clippy
  found `f32::hypot` in the drainage (ADR-0032), now `libm::hypotf`, which
  left every digest test passing.
- `worldgen_v3` against `main` (454d4f96), 7 interleaved rounds, load 5-10, CPU
  per Chunk: straddling dry +7.7 % at 1 m and +7.2 % at 0.5 m, straddling
  crop field +4.3 % at 1 m, every other case within +/-1.5 %. Straddling dry is
  ticket 04's +8.3 %, still over the 5 % gate and still the user's to decide.
- 2026-11-27, after tickets 17-28. `worldgen_v3` straddling dry was +13% on
  `main`. Not content: the dry Chunk's ground is the same on both. A sampled
  profile of that case alone put it in `wet`, which bends every stream segment
  in the stencil with four noise samples where `main` bent the column once. An
  inland segment too far to reach the column however it bends is now skipped
  before its noise, which leaves every recorded Chunk unchanged. Seven
  interleaved rounds against `main`: dry +4.6% @ 1 m and +0.6% @ 0.5 m, every
  other case within 2.4% or faster. Full suites: core 1194 and game 2001 pass.
- 2026-11-27: the user reviewed the captures and approved them.
- 2026-11-27: the user accepted the budget: the water path's GPU cost within
  0.5 ms everywhere, the median frame +0.70 ms at the lake and +0.85 ms
  under water.
