# 02: Coarse agreement

Status: done
Blocked by: 01

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md).

## What to build

Flying toward a coast, the shore stays where it is. Every coarse level answers
water or land, and its top Material, as level 0 does at each column's centre in
metres: the same terrain height against that column's own water level, sea or
lake, the Biome sampled at the centre. A dry column's top is raised to at least
that water level. A column under an Edit or saved change keeps the downsample's
answer. This replaces the per-level half-Voxel lowering and its tie, so level 4
draws its shallow sea again. At levels 1 and 2 it applies after the downsample.

The rule lives in the coarse path only: `Worldgen` at a Voxel size is also level
0 for a World of that size, and version 3's output is frozen.

## Acceptance criteria

- [x] 01's agreement test asserts exact agreement on water or land and top
      Material at every sampled centre, levels 1 to 5.
- [x] Every existing worldgen, determinism and golden test passes unchanged.
- [x] A Chunk with an Edit or saved change still shows the change at every level
      it did.
- [x] Coarse-Chunk CPU time per level from an interleaved micro-benchmark
      against the integration branch's tip, in Comments.
- [x] Captures of a coast at each ring join, for the review batch after 05.

## Comments

2026-11-22: Done on `t3/coarse-agreement`.

What changed. `Worldgen::column_top` answers a level-0 column cache-free (its
own output is untouched); `agree_with_level_zero` in `tesserae-core` rewrites
each coarse column from that answer at its centre; `server/lod.rs` applies it to
every untouched coarse Chunk (after the downsample at 1 and 2, over the column
generators from 3) and to the saved-change path, skipping columns under a
changed child. The column generators lose the half-Voxel lowering and its tie.
The surface sits in the coarse Voxel whose middle it passes (the vote's
rounding); water fills each Voxel it reaches into, which the mesh job then
levels to level 0's surface as before; a dry top is raised to the waterline
Voxel. Each Voxel is decided by metres, so a column's Chunks agree without
reading each other; only the gap under a moved top is filled from the Chunk's
own Voxels. A coarse Chunk column's 1,024 answers are kept for its other rows
(256 columns, 16 KB each).

Agreement test (01's harness, now asserting): water or land and top Material
exact at every sampled centre, levels 1 to 5, coast and forest windows. Columns
whose level-0 top lies below their own terrain surface, a cave mouth or shaft
opening level 0, are counted and not compared: no coarse level draws a 1 m
opening, and from level 3 up none has caves. Before that exclusion level 1
differed on 0.6% of the coast window's tops, all of them such columns.

| Coast | columns | wet/dry differs | top Material differs | level 0 carved |
| ----- | ------- | --------------- | -------------------- | -------------- |
| 1     | 16060   | 0.0%            | 0.0%                 | 2.0%           |
| 2     | 4014    | 0.0%            | 0.0%                 | 2.0%           |
| 3     | 1003    | 0.0%            | 0.0%                 | 2.1%           |
| 4     | 251     | 0.0%            | 0.0%                 | 2.0%           |
| 5     | 62      | 0.0%            | 0.0%                 | 3.1%           |

| Forest | columns | wet/dry differs | top Material differs | level 0 carved |
| ------ | ------- | --------------- | -------------------- | -------------- |
| 1      | 15795   | 0.0%            | 0.0%                 | 3.6%           |
| 2      | 3952    | 0.0%            | 0.0%                 | 3.5%           |
| 3      | 984     | 0.0%            | 0.0%                 | 3.9%           |
| 4      | 244     | 0.0%            | 0.0%                 | 4.7%           |
| 5      | 60      | 0.0%            | 0.0%                 | 6.2%           |

Tests. `cargo test -p tesserae-core` (1,065 pass, every worldgen, determinism
and golden test unchanged) and `cargo test -p tesserae` (1,654 pass, 4 ignored);
`cargo clippy -p tesserae -p tesserae-core --all-targets --all-features` clean.
Seven LOD tests compared held coarse Chunks with the raw downsample or column
generator; their expectation now agrees the same columns, and keeps an edited
child's. Two of them inserted a level-0 Chunk without marking its Voxels
changed, which production always does; they now mark it. The Edit, Mod fill and
saved-change tests pass with the edited columns unagreed. One unrelated
`open::tests` test failed once under load and passes alone.

Coarse-Chunk CPU time. Thread CPU per Chunk, median of 9 interleaved rounds in
mirrored order, cold caches every pass, Terra at its declared 160 m / 3000 m, a
3 x 3 (levels 1-2) or 4 x 4 Chunk-column block by spawn, every row of the
terrain band. A is the integration tip's job (`88bc4978`: downsample, or the
lowered column generator), B this branch's, in one test binary
(`optimized + debuginfo`). Host loaded by parallel builds: 1-minute load 11-12.
Two runs:

| Level | Chunks | A ms     | B ms      | B / A      |
| ----- | ------ | -------- | --------- | ---------- |
| 1     | 63     | 8.5, 6.3 | 8.9, 6.6  | 1.05, 1.03 |
| 2     | 36     | 9.7, 6.9 | 9.6, 7.2  | 0.99, 1.05 |
| 3     | 48     | 2.1, 1.9 | 4.0, 2.9  | 1.92, 1.50 |
| 4     | 32     | 4.7, 3.0 | 7.8, 5.0  | 1.67, 1.69 |
| 5     | 32     | 7.1, 4.8 | 11.0, 7.2 | 1.55, 1.48 |

- Levels 1 and 2 are within noise of the tip; levels 3 to 5 cost 1 to 4 ms more
  per Chunk in the terrain band. A level-0 answer costs about what a column-only
  coarse Chunk spends on a column: one terrain column and the four neighbour
  water levels its containment reads. Without the per-column cache levels 1 and
  2 were +19-27%. Rows outside the terrain band skip the rule.
- The full budget series is 09's.

Captures (not committed, for the review batch after 05):
`/tmp/lod22-02-coast-2IgxO2`. The coast at Chunk columns `[43, -31]`, Eye 160,
320, 640, 1280 and 2560 m west of the shore at z = -863.5 looking east, so each
ring join falls on it: `before-join-*.png` from the integration tip
(`88bc4978`), `after-join-*.png` from this branch, `dev` build, offscreen 1920 x
1080, `stationary` fixture with no flight.
