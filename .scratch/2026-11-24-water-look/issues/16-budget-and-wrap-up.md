# 16: Budget and wrap-up

Status: done
Blocked by: 11, 12, 13, 14, 15

Spec: [Water that looks natural](../spec.md), Budget.

## What to build

Measure the finished water against `main` and close the spec.

## Acceptance criteria

- [ ] Frame timing and a `--instruments gpu` series at the four water fixtures
      and `flight`, against `main` with 01's fixture commit cherry-picked: each
      water fixture at most +0.5 ms median frame, `flight` no worse beyond
      noise. Numbers in Comments.
- [ ] Final captures from every fixture, before and after, shown to the user.
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
