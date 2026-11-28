# 03: Meanders that grow with the river

Status: done
Blocked by: 02

Spec: [Water that looks natural](../spec.md), Hydrology.

## What to build

Large rivers swing in wide bends and brooks in tight ones: the meander warp's
amplitude and period scale with river width. Today the warp is taken once per
point before the segments are walked; scaling it by width means a warp per
segment in reach, so this ticket measures that cost. The amplitude stays inside
the valley's reach, so it still fits the drainage stencil. Generator version 3
changes in place; new code uses no platform maths (ADR-0032).

## Acceptance criteria

- [x] A test: meander amplitude grows with river width, and no river's valley
      reaches past the stencil (no cut ends at a region border).
- [x] "A river valley is not a cliff across a region border" and the other
      terrain tests pass.
- [x] Digests recorded again; the `worldgen_v3` bench in Comments, under the
      same 5 % gate as 02.
- [x] Before and after captures of a large river and a brook, shown to the user.

## Comments

- 2026-11-26, on 9a2fcae4 plus this ticket:
  - Each stream swings `2 x width` to either side, 3 m for a brook up to the
    old 14 m cap from 7 m wide, so the stencil and valley reach are unchanged.
    Its bends blend from tight (40 m noise) to long (240 m) as it widens to
    20 m. Both noises are at fixed sizes: a size that varied with width
    stretched the noise about the origin and stepped the ground 2.2 m within a
    quarter metre 2.8 km out (`the_ground_has_no_step_anywhere_along_a_line`).
  - The warp is per segment, blended between its two ends as width is, so a
    tributary and its river bend alike where they meet.
  - Digests moved: `coarse_generation` (all 9) again.
  - `worldgen_v3` against `main`, 9 interleaved rounds, load 6-9: at most
    +3.9 % (straddling dry at 1 m); river -1.8 % at 1 m, -2.8 % at 0.5 m.
  - The `river` fixture's river moved with its new bends; its Eye moved to
    (-4061.3, 40.6, 448.7), yaw 135. Captures `river-before-03`,
    `river-after-03`, `slope-before-03` and `slope-after-03` in
    `/tmp/tesserae-water-look-captures/`. No fixture shows a river wider than
    about 6 m; 04's mouth fixture is the first large one.
