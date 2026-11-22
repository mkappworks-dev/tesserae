# 03: Tree Impostors at levels 1 and 2

Status: done
Blocked by: 02

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md).

## What to build

Trees stand on the ground at levels 1 and 2, exactly where level 0 stands them,
and do not pop in at the join.

- **The placement generator.** The coarse jobs share a second level-0 generator
  with the World's seed, parameters, features and Templates. It answers tree,
  Structure and Settlement draws only, never a Chunk. It shares its 1 m column
  cache with level 1's source, and that pair gets its own caches, sized for
  level 1's ring, instead of level 0's, which nothing coarse then touches.
- **Level 2 measured first.** Its children are 2 m, so its draws find no warm
  1 m columns. If a level-2 Chunk's draws cost past the per-level budget, level
  2 takes Canopy cover (05) instead, recorded in the spec.
- **Impostors.** Each tree level 0 stands in a coarse Chunk's footprint, fully
  grown, is written into the coarse grid so that any trunk or canopy Voxel
  beats air; canopy wins over trunk, either over ground. Level 1 always runs
  the draw, whatever the Server holds.
- **Saved changes.** A branch holding an Edit or saved change runs no draw: its
  changed level-0 Voxels are coarsened with the same any-solid rule for tree
  Materials, so a felled tree stays felled and a standing one keeps its shape.

## Acceptance criteria

- [x] The placement generator's tree draws equal level 0's for the same region.
- [x] A level-1 Chunk built from held children equals one built from generated
      children, Edits aside.
- [x] A tree's Impostor occupies the coarse Voxels its level-0 trunk and canopy
      reach.
- [x] A felled tree whose Chunk is evicted is absent at level 1 in a persistent
      World.
- [x] Coarse-Chunk CPU time per level from an interleaved micro-benchmark
      against the integration branch's tip, in Comments, with
      vertex bytes per level from a test, and level 0's column-cache hit rate
      unchanged.
- [x] The level-2 decision, Impostors or Canopy cover, with its numbers.
- [x] Captures of a forest edge across the level 0 to 1 and 1 to 2 joins, for
      the review batch after 05.

## Comments

2026-11-22: Done on `t3/tree-impostor-agreement`. Level 2 takes Canopy cover
(05), not Impostors: the spec records it.

What changed.

- `Worldgen::tree_parts(min, max)` (core) is the draw: every trunk and canopy
  Voxel, fully grown, that level 0 stands in a box of global cells, fruit as
  canopy, kept only where level 0 would put it (over the column's top, out of
  its lake, off a smoothing stair), trunk over canopy in a shared cell, sorted
  so two species meeting in one coarse Voxel always resolve alike. Version 3's
  output is untouched: every core test and golden passes unchanged.
- `stand_tree_parts` and `coarsen_tree_parts` (core `lod`) write a coarse
  Chunk: each part makes its coarse Voxel its Material whatever the vote and
  agreement left, canopy over trunk; a changed octant draws nothing and takes
  any trunk or canopy among its finer Voxels instead, so a felled tree stays
  felled at every level a change climbs to.
- `LodGenerators` holds the placement generator: level 0's generator cloned
  with its Templates and Settlements and caches of its own
  (`Worldgen::with_own_caches`), its column cache sized for level 1's ring at
  the Detailed unit (52 x 52 Cells, about 24 KB each, filled only where level 1
  streams). Level 1's source shares that cache, not level 0's, so nothing coarse
  touches level 0's caches; a test asserts no coarse generator shares it. Level
  0's column-cache hit rate can therefore only rise; it is not counted.
- A level-1 job reads only changed held children: an unchanged one is what the
  bare generator makes plus the draw, so held and generated give one Chunk.
- `IMPOSTORS_TO` is 1.

Tests (`server::lod::tests::trees`, `server::lod::agreement`, core
`plant_worldgen`, core `lod`): the draw equals level 0's trunk and canopy
Voxels over a box with fruit, a branching species and smoothing; the Impostor
occupies exactly the coarse Voxels level 0's tree Voxels reach; a level-1 Chunk
from held children equals one from generated ones; a felled tree is absent at
level 1 while held and after its Chunk is saved and evicted in a persistent
World; the placement and level-1 generators share a cache level 0 does not.

Agreement on Terra (01's harness): ground still exact at levels 1 to 5 (read
without Impostors). Trees per coarse column, against any level-0 column of its
footprint standing a tree (level 2 shown with Impostors forced on, for the
decision; it ships none):

| Window | Level | columns | level 0 tree in footprint | stood | missing | extra |
| ------ | ----- | ------- | ------------------------- | ----- | ------- | ----- |
| coast  | 1     | 16060   | 11.3%                     | 11.4% | 0.00%   | 0.09% |
| coast  | 2     | 4014    | 15.2%                     | 15.2% | 0.00%   | 0.07% |
| forest | 1     | 15795   | 29.3%                     | 29.6% | 0.00%   | 0.26% |
| forest | 2     | 3952    | 36.9%                     | 37.1% | 0.00%   | 0.13% |

The test now asserts no column misses a tree and at most 1% stand one level 0
does not, at level 1. An extra is a cell level 0 gave to something else first,
a ground-cover tuft or an overhang the draw does not see.

Coarse-Chunk CPU time. Thread CPU per Chunk, median of 9 interleaved rounds in
mirrored order, cold caches every pass (fresh generators, terrain regions
forgotten), Terra at its declared 160 m / 3000 m, a 3 x 3 Chunk-column block
over the forest window's middle, every row of the terrain band
(`agreement::coarse_chunk_cpu_time_and_vertex_bytes_with_and_without_impostors_over_terra_forest`,
`--ignored`, `optimized + debuginfo`). "Without" is the integration tip's job
for an untouched coord (downsample, then agreement), "with" adds the draw.
Host load 4-6. Three runs:

| Level | Chunks | without ms       | with ms             | with / without   |
| ----- | ------ | ---------------- | ------------------- | ---------------- |
| 1     | 63     | 4.10, 4.13, 4.11 | 10.52, 10.88, 10.57 | 2.56, 2.63, 2.57 |
| 2     | 36     | 4.57, 4.55, 4.56 | 32.74, 32.66, 32.90 | 7.17, 7.19, 7.21 |

- Level 2's draw adds 28 ms a terrain Chunk: its 128 m footprint and reach
  span some 36 cold 32 m Cells of Plants, whose draws cost 0.5-0.9 ms each cold,
  and nothing at level 2 warms them. Against level 1's 6 ms, and the step-6
  table's 0.6-0.7 ms mean job, it is past the budget, so level 2 takes Canopy
  cover.
- Level 1 is 2.6 times its job over the densest forest; it pays only in the
  terrain band and only where trees stand. The fill-time effect is 09's series.

Vertex bytes, the same block meshed with its neighbours (one run each; the
first run's level-1 figure, 354,480, differed by 0.3% before the draw's order
was fixed):

| Level | meshes | without bytes | with bytes |
| ----- | ------ | ------------- | ---------- |
| 1     | 63     | 131,440       | 353,216    |
| 2     | 36     | 103,440       | 329,344    |

A forest's level-1 mesh is 2.7 times its bare ground's; the whole-field figure
is 09's.

Captures (not committed, for the review batch after 05):
`/tmp/lod22-03-trees-BPBk`. Eye over the forest window's west edge looking
east (yaw -90), `before-*` from the integration tip (`9587aad8`), `after-*`
from this branch, `dev` build, offscreen 1920 x 1080, no flight:
`join-0-1-forest` (Eye 1500.5, 75.5, -735.5, pitch -14),
`join-0-1-low` (50.5 m, pitch -4) and `join-1-2-forest` (Eye 1340.5, 105.5,
pitch -12), so the level 0 to 1 and 1 to 2 joins fall on the forest. At 1 to 2
the trees end at the join until 05.

Ran: `cargo fmt --all`, `npm run fmt`, `cargo test -p tesserae-core` (1,068
pass), `cargo test -p tesserae --bins` (1,669 pass, 4 ignored), `cargo clippy
-p tesserae -p tesserae-core --all-targets --all-features`.


2026-11-22, follow-up: a level-1 job inside level 0's load generates all
eight children, as it does outside; `a_level_one_job_counts_...` (a `dev` test
the `--bins` run missed) now states that rule. Reading an unchanged held child's
terrain instead is not exact: a held Chunk carries its Structures, which also
keep the overhang density off the ground round them, so stripping trees and
ground cover does not give back the bare Chunk, and held would no longer equal
generated. What the generation costs, Impostors drawn either way, CPU time per
Chunk, median of 9 interleaved rounds in mirrored order, cold coarse caches
every pass, a 3 x 3 level-1 Chunk-column block by Terra's spawn, every row of
the terrain band
(`agreement::level_one_cpu_time_generating_children_against_reading_held_ones_by_terra_spawn`,
`--ignored`). Host heavily loaded (1-minute load 25 to 34), two runs:

| Chunks | reading held ms | generating ms | extra ms   |
| ------ | --------------- | ------------- | ---------- |
| 63     | 13.30, 13.20    | 19.37, 18.82  | 6.07, 5.63 |

About 6 ms a terrain-band level-1 job, only for the level-1 jobs over level
0's load; 09's fill-time series shows whether it moves level 0's fill.

Ran: `cargo test --workspace --locked --features tesserae/dev` (3,103 pass, 10
ignored), `cargo clippy --workspace --all-targets --all-features` (clean),
`cargo fmt --all`.
