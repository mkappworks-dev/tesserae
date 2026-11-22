# 05: Canopy cover from level 2 up

Status: done
Blocked by: 02, 03

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md).

## What to build

Forests read as forests past the trees' last level. From level 2 up, a coarse column's top
Voxel becomes the dominant species' leaf Material where the expected tree cover
is over half: the Biome's tree chance times the species' canopy area times its
clump noise, read from the placement generator at the column's centre, and zero
under water, on Structure footprints and on fields. It is raised one coarse
Voxel above the ground where the species' mean canopy top is at least half that
Voxel, so forests keep a silhouette until their Voxel outgrows the trees.

From level 2, not 3 as first planned: ticket 03 measured level 2's Impostor
draws at 33 ms a terrain Chunk against level 1's 11 ms, so level 1 keeps 03's
tree Impostors and the level 1 to 2 join goes from trees to Canopy cover.

## Acceptance criteria

- [x] A forest's edge agrees with level 0's tree density within one coarse
      column, in 01's agreement test.
- [x] No Canopy cover on water, Structures or fields.
- [x] Coarse-Chunk CPU time per level from an interleaved micro-benchmark
      against the integration branch's tip, in Comments.
- [x] Captures of forest and open ground across the level 1 to 2 and 2 to 3
      joins and at 2 and 4 km; with 02 to 04's, the first review batch, shown to you.

## Comments

2026-11-22: Done on `t3/lod-canopy-cover-level-two`, merged with the
integration branch after ticket 04.

What changed.

- `Worldgen::canopy_shapes` (core) grows each tree feature 16 times at level
  0's Voxel size, fully grown, for its mean crown area (every column a trunk,
  canopy or fruit Voxel stands over) and mean height over the ground.
  `Worldgen::canopy_cover` reads, at a column's centre, the Biome there and
  each tree feature's share of the draw, clump included, as a density;
  thins it by crowding as a draw-ordered hard core does (`(1 - e^-x) / x` of
  the trees within the crowding radius); multiplies by the crown area; and
  takes `1 - e^-sum` as the share covered, since crowns still overlap. Over
  half, it answers the dominant species' leaves and height. Version 3's output
  is untouched: every core test and golden passes unchanged.
- None stands on a field of the column's Biome, a Settlement's footprint, or
  where any Structure candidate of a cell's draw would stand: deciding which
  candidates the ground takes cost 100 to 450 ms a level-4 or 5 Chunk, and a
  candidate is about a coarse column, so the check is by hashes alone and
  leaves out a few columns a rejected candidate would have had. Crossings are
  left out of the check: they stand over water. Water is the coarse level's to
  rule out, from the agreed answer.
- `stand_canopy_cover` (core `lod`) writes the leaves on the agreed top, the
  row `agree_with_level_zero` gives it, or one coarse Voxel over it where the
  species' mean height reaches half a Voxel. On Terra every tree stands 8 m or
  more, so the cover is raised at levels 2 to 4, and at 5 only for the tallest
  species; the spec's "silhouette at 8 m, tint at 16 and 32 m" assumed shorter
  trees, and the rule, not the example, is what shipped.
- `make_coarse` and `generate_persistent` draw it above `IMPOSTORS_TO`, after
  agreement and before 04's Structures, which beat it. A changed child's
  columns are left out by agreement and keep their own tree Voxels by 03's
  any-solid rule, now applied at every level, so a felled forest stays bare.

Tests: core `lod` (the agreed top or the Voxel over it, never water, only in
the Chunk holding that row), `plant_worldgen` (a dense forest covers, a sparse
one does not), `fields` and `structure_keep_off` (none on a field or a
Structure that would otherwise be covered), `server::lod::tests::canopy` (one
coarse Voxel of leaves over the ground at levels 2 to 4; a felled level-0
Chunk stays bare at levels 2 to 4 while the forest round it is covered), and
01's agreement test now asserts no cover on water and that cover differs from
level 0's tree density past one coarse column on at most 2% of columns.

Agreement on Terra (01's harness, a third window found where Canopy cover
stands). "Level 0 trees over half": columns over half of whose footprint level
0 stands a tree over. "Past one column": columns that differ from every column
within one of them.

| Window | Level | columns | level 0 trees over half | Canopy cover | differs | past one column |
| ------ | ----- | ------- | ----------------------- | ------------ | ------- | --------------- |
| coast  | 2     | 4014    | 8.0%                    | 10.1%        | 8.5%    | 0.52%           |
| coast  | 3     | 1003    | 8.3%                    | 10.5%        | 9.0%    | 0.20%           |
| coast  | 4     | 251     | 7.2%                    | 10.0%        | 7.6%    | 0.00%           |
| coast  | 5     | 62      | 6.5%                    | 11.3%        | 4.8%    | 0.00%           |
| forest | 2     | 3952    | 22.9%                   | 21.1%        | 12.8%   | 0.38%           |
| forest | 3     | 984     | 24.0%                   | 21.1%        | 11.8%   | 0.20%           |
| forest | 4     | 244     | 20.5%                   | 20.9%        | 7.8%    | 0.41%           |
| forest | 5     | 60      | 20.0%                   | 18.3%        | 5.0%    | 0.00%           |
| canopy | 2     | 3976    | 53.9%                   | 64.5%        | 23.1%   | 0.93%           |
| canopy | 3     | 991     | 56.7%                   | 64.7%        | 21.3%   | 0.40%           |
| canopy | 4     | 245     | 56.7%                   | 64.9%        | 18.0%   | 0.41%           |
| canopy | 5     | 62      | 59.7%                   | 62.9%        | 12.9%   | 0.00%           |

Summing crown areas without the overlap term covered 68.6% of the canopy
window at level 2 against level 0's 53.9%, and differed past one column on
1.31%; the overlap term is what shipped. Where level 0's density sits near
half, a 4 m column's 16 level-0 columns are a noisy sample, which is most of
the "differs" column.

Coarse-Chunk CPU time. Thread CPU per Chunk, median of 9 interleaved rounds in
mirrored order, cold caches every pass, Terra at 160 m / 3000 m, a 3 x 3
Chunk-column block over each window's middle, every row of the terrain band
(`agreement::coarse_chunk_cpu_time_and_vertex_bytes_with_and_without_trees_over_terra_forest`,
`--ignored`, release). "Without" is the integration tip's job (`2adf87e3`,
with 04's Structures) for an untouched coord, "with" adds Canopy cover (level
1: 03's Impostors, unchanged by this ticket). Host load 3 to 19. Three runs,
all within 0.03 ms of each other; the middle one:

| Window | Level | Chunks | without ms | with ms | with / without |
| ------ | ----- | ------ | ---------- | ------- | -------------- |
| forest | 2     | 36     | 3.33       | 3.39    | 1.02           |
| forest | 3     | 27     | 3.35       | 3.42    | 1.02           |
| forest | 4     | 18     | 8.39       | 8.49    | 1.01           |
| forest | 5     | 18     | 15.46      | 15.50   | 1.00           |
| canopy | 2     | 36     | 3.43       | 3.52    | 1.03           |
| canopy | 3     | 27     | 3.35       | 3.42    | 1.02           |
| canopy | 4     | 18     | 8.39       | 8.49    | 1.01           |
| canopy | 5     | 18     | 15.44      | 15.49   | 1.00           |

- Canopy cover adds 0.05 to 0.1 ms a terrain Chunk at every level: the cover
  itself is about 0.3 to 0.5 ms of arithmetic over 1,024 columns, and 04's
  Structure pass has already warmed the Settlement layouts it reads.
- Before 04 merged, against `ea48f268`, it added 0.3 ms at level 2 and 0.6 to
  1.8 ms at 3 to 5 (cold Settlement layouts), and 20 to 160 ms at levels 3 to 5
  in its first form, which asked which Structures the ground takes.
- Level 4 and 5's growth from 3 to 15 ms in "without" is 04's.

Vertex bytes, the same blocks meshed with their neighbours:

| Window | Level | meshes | without bytes | with bytes |
| ------ | ----- | ------ | ------------- | ---------- |
| forest | 2     | 36     | 110,496       | 133,440    |
| forest | 3     | 27     | 57,040        | 81,488     |
| forest | 4     | 18     | 60,976        | 81,728     |
| forest | 5     | 18     | 42,848        | 61,168     |
| canopy | 2     | 36     | 158,160       | 196,928    |

A raised cover adds the sides of every step at its edge: 21% to 43% more
vertex bytes over a forest. The whole-field figure is 09's.

Captures (not committed, for the review batch): `/tmp/lod22-05-canopy-pLkO`.
`before-*` from the integration tip before 04 (`ea48f268`), `after-*` from this
branch before the merge with 04, so neither shows Structure Impostors; `dev`
build, offscreen 1920 x 1080, no flight: `join-1-2-forest` (Eye 1340.5, 105.5,
-735.5, yaw -90, pitch -12), `join-2-3-forest` (Eye 1120.5, 135.5, -735.5,
pitch -9), `at-2km` and `at-4km` (Eye 1760.5, 300.5 or 420.5, 1264.5 or
3264.5, yaw 0, looking north at the forest windows). At the level 1 to 2 join
trees give way to a 4 m leaf layer standing a Voxel over the ground: the forest
keeps its place and colour, but drops from trees 12 to 25 m tall to one 4 m
step. At 2 km the forests that read as bare ground on the tip are green. At 4
km the view is past Terra's 3 km reach and shows fog.

Left open: ADR-0035 still says Impostors stand at levels 1 and 2 and Canopy
cover from level 3; the spec records the change.

Ran: `cargo fmt --all`, `npm run fmt`, `cargo test -p tesserae-core`, `cargo
test -p tesserae --bins` (1,688 pass, 6 ignored), `cargo clippy -p tesserae -p
tesserae-core --all-targets --all-features`.
