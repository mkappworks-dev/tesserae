# 11: Canopy cover with relief

Status: done
Blocked by: 05

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md).

## What to build

In the first capture review, 05's Canopy cover read as a flat green sheet laid
over the terrain's steps, and the level 1 to 2 join changed abruptly from single
trees to that sheet. Break the sheet up so a forest reads as a mass of trees:
denser columns stand one coarse Voxel higher than sparser ones, and each
column's leaf shade varies a little with its cover, both from the same cover
value 05 already computes at the column's centre. Nothing is stored, and it
stays a pure function of seed and metres.

## Acceptance criteria

- [x] A dense forest's top has more than one height at levels 2 to 4; a column
      with the same cover always gets the same height and shade.
- [x] 05's exclusions and its agreement bound still hold.
- [x] Coarse-Chunk CPU time per level from an interleaved micro-benchmark
      against the integration branch's tip, in Comments: under 0.1 ms per
      Chunk added.
- [x] Before and after captures at the level 1 to 2 and 2 to 3 joins and at
      2 km, shown to you.

## Comments

2026-11-22: Done on `t3/canopy-relief-lod`, from the integration tip
`24c35110`.

What changed.

- `canopy_stand` (core `lod`) decides each column's height and leaves from its
  `Canopy` alone. A column covered three quarters or more (`DENSE_COVER`) fills
  the coarse Voxels over its ground that its trees' mean height reaches at least
  half of, up to two (`CANOPY_RISE`); a sparser one stands a Voxel lower. Where
  the trees reach under half a Voxel, as at level 5, both are the top Voxel
  itself and only the shade differs. `stand_canopy_cover` fills every Voxel from
  the ground to that height, and `canopy_candidates` takes the rows up to two
  Voxels over the top.
- `Canopy` gains `sparse`, the leaves a sparser column shows: the next
  species' where its trees cover at least half what the dominant species' do,
  else the dominant's. `expected_canopy` already summed each species' share; it
  now keeps them sorted, the dominant first as before. No Material is added:
  Terra's forests all mix two species (oak and birch, pine and birch, jungle and
  palm) except the swamp, whose sparser columns differ by height only. A darker
  variant of one species' leaves would need a renderer tint or new Materials.
- 05's exclusions, saved-change rule and Structure order are untouched;
  Generator v3's output is too (every core test passes unchanged).

Tests: core `lod` (`canopy_stand` at 4, 16 and 32 m Voxels; a dense column
fills two Voxels and a sparse one one in its own shade, across a Chunk row),
`plant_worldgen` (a mixed forest's sparser shade is the next species' only where
it covers at least half, and a column's answer does not depend on the columns
asked beside it), `server::lod::tests::canopy` (each covered column stands where
`canopy_stand` says, two 4 m Voxels, one 8 m, the top at 16 m for the fixture's
7 m trees), and 01's agreement test now prints Canopy cover's columns by height
and leaves and asserts more than one height over the canopy window at levels 2
to 4. Its agreement rates are 05's to the column.

Canopy cover's columns by Voxels over the ground, and by leaf Material, over
the agreement windows:

| Window | Level | 0     | 1     | 2     | leaves (Material: columns) |
| ------ | ----- | ----- | ----- | ----- | -------------------------- |
| forest | 2     |       | 394   | 439   | 41: 648, 29: 112, 38: 73   |
| forest | 3     |       | 97    | 111   | 41: 165, 29: 25, 38: 18    |
| forest | 4     | 21    | 24    | 6     | 41: 39, 29: 6, 38: 6       |
| forest | 5     | 10    | 1     |       | 41: 9, 29: 1, 38: 1        |
| canopy | 2     |       | 1,366 | 1,199 | 41: 2,393, 29: 99, 38: 73  |
| canopy | 3     |       | 334   | 307   | 41: 601, 29: 22, 38: 18    |
| canopy | 4     | 83    | 70    | 6     | 41: 146, 29: 7, 38: 6      |
| canopy | 5     | 38    | 1     |       | 41: 37, 29: 1, 38: 1       |

Coarse-Chunk CPU time. 05's measurement
(`agreement::coarse_chunk_cpu_time_and_vertex_bytes_with_and_without_trees_over_terra_forest`,
`--ignored`, release: thread CPU per Chunk, median of 9 interleaved rounds in
mirrored order, cold caches every pass, a 3 x 3 Chunk-column block over each
window's middle) run from two release test binaries, the tip's (A) and this
branch's (B), in the order A B B A. Load average 5.6 falling to 1.5. "Cover"
is with less without in the same run, so each run's drift cancels; the table
gives the mean of each binary's two runs.

| Window | Level | Chunks | A with ms | B with ms | A cover ms | B cover ms | added ms |
| ------ | ----- | ------ | --------- | --------- | ---------- | ---------- | -------- |
| forest | 2     | 36     | 3.370     | 3.345     | 0.060      | 0.055      | -0.005   |
| forest | 3     | 27     | 3.380     | 3.400     | 0.070      | 0.075      | 0.005    |
| forest | 4     | 18     | 8.440     | 8.455     | 0.105      | 0.105      | 0.000    |
| forest | 5     | 18     | 15.425    | 15.460    | 0.065      | 0.045      | -0.020   |
| canopy | 2     | 36     | 3.495     | 3.490     | 0.095      | 0.110      | 0.015    |
| canopy | 3     | 27     | 3.385     | 3.400     | 0.075      | 0.075      | 0.000    |
| canopy | 4     | 18     | 8.420     | 8.445     | 0.095      | 0.095      | 0.000    |
| canopy | 5     | 18     | 15.405    | 15.430    | 0.040      | 0.040      | 0.000    |

- Relief and shade add nothing measurable: within 0.02 ms a Chunk either way,
  the runs' own spread.

Vertex bytes, the same blocks meshed with their neighbours:

| Window | Level | meshes | without | 05's cover | with relief |
| ------ | ----- | ------ | ------- | ---------- | ----------- |
| forest | 2     | 36     | 110,496 | 133,440    | 142,736     |
| forest | 3     | 27     | 57,040  | 81,488     | 94,448      |
| forest | 4     | 18     | 60,976  | 81,728     | 98,288      |
| forest | 5     | 18     | 42,848  | 61,168     | 61,168      |
| canopy | 2     | 36     | 158,160 | 196,928    | 216,448     |

- The steps between dense and sparse columns add 7% to 20% over 05's cover at
  levels 2 to 4; level 5 is unchanged.

Captures (not committed): `/tmp/lod22-11-relief-iM2b`. `before-*` from the
integration tip `24c35110`, `after-*` from this branch; `dev` build, offscreen
1920 x 1080, no flight, 05's Eyes: `join-1-2-forest` (1340.5, 105.5, -735.5,
yaw -90, pitch -12), `join-2-3-forest` (1120.5, 135.5, -735.5, pitch -9),
`at-2km` (1760.5, 300.5, 1264.5, yaw 0, pitch -6). At both joins the forest's
top steps between one and two coarse Voxels in clumps the size of the clump
noise, where the tip lays one sheet; at 2 km the island's forest stands taller
and broken. Both builds' join captures show the same holes in the near ground
(caves and the sea floor seen through missing level-0 Chunks), so they are the
tip's, not this ticket's.

Ran: `cargo fmt --all`, `npm run fmt`, `cargo clippy --workspace --all-targets
--all-features`, `cargo test --workspace --locked --features tesserae/dev` (all
pass; the `tesserae` bin 1,729 and 8 ignored). One earlier run at load average
15 failed `persistence::tests::an_unchanged_wet_chunk_leaves_mid_rescan_without_a_commit_and_settles_on_return`
with "step is running"; it passed alone three times and in the rerun, and
touches nothing this ticket changed.
