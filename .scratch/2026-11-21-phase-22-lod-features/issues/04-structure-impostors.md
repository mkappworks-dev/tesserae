# 04: Structure Impostors

Status: done
Blocked by: 03

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md),
[ADR-0029](../../../docs/adr/0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md).

## What to build

Buildings and villages show on the horizon. Every Structure and Settlement Plot
that level 0 stands is sampled at each coarse level's Voxel size, any solid
beating air, from 03's placement generator. Settlement paths are ground
Material. Over saved changes, the changed Voxels are used, as in 03.

## Acceptance criteria

- [x] The placement generator's Structures and Plots equal level 0's for the
      same region.
- [x] A Structure's Impostor occupies the coarse Voxels its level-0 Voxels
      reach, at every coarse level.
- [x] A Creative placement or removal, once saved and evicted, shows at the
      coarse levels as it does today.
- [x] Coarse-Chunk CPU time per level from an interleaved micro-benchmark
      against the integration branch's tip, in Comments.
- [x] Captures of a Settlement at 0.5, 1 and 2 km, for the review batch after
      05.

## Comments

2026-11-22: Done on `t3/structure-impostors`. Bridges stand at level 1 only:
the spec records it.

What changed.

- `Worldgen::structure_parts(min, max, spans)` (core) is the draw: every solid
  Voxel a single Structure, its foundation and ground cells, or a Settlement
  Plot's Structure writes in a box of global cells, in level 0's write order,
  from the same claims. `stand_structure` now writes through
  `structure_voxels`, which the draw shares, so the two cannot drift. Version
  3's output is untouched: every core test and golden passes unchanged.
- `Worldgen::paths(columns)` (core) answers the path Material a road or trail
  lays as a column's top, none on a worn lane, the last Settlement's where two
  meet. The coarse agreement takes it as the top Material at each column
  centre, so paths are ground: no Impostor, no raise.
- `stand_structure_parts` and `coarsen_structure_parts` (core `lod`) write a
  coarse Chunk, as 03's tree writers do: each part makes its coarse Voxel its
  Material whatever the vote, agreement or a tree left; a changed octant draws
  nothing and keeps any built Voxel among its finer Voxels. "Built" is
  `Worldgen::built_materials`, every Template Voxel and foundation Material no
  Biome lays as ground or water, so ground over a change stays the vote's.
- "Any solid" is the Material table's `solid`: a Plot's crops, flowers and
  torches stay out, as Plants do (Q4). Built parts stand after a Template's
  ground-coloured cells, so a wall beats the soil a Plot lays in one coarse
  Voxel; without it 0.2% of footprints at level 1 lost their floor to dirt.
- A Plot's cut and fill is ground, like paths: the coarse ground keeps level
  0's natural height, and a floor cut into a slope can sit one coarse Voxel
  under the ground's top at its uphill edge.
- `SPANS_TO` is 1: a spanning Template (a bridge) stands at level 1 only. Its
  lattice cell's gap search reads up to every metre of a 96 m cell's ground;
  over level 1's warm 1 m columns that is free, above it some 3 s of CPU a km²
  cold. Cutting it from levels 2 to 5 took a level-5 Settlement Chunk from 1.5
  s to 31 ms. A later row still gives way to a bridge, so every other
  Structure stands as level 0 stands it.
- The placement generator draws Structures at every coarse level
  (`Makers::structures`), over saved changes too (`generate_persistent`).

Tests: core `structure_worldgen`, `settlement_worldgen` and `bridge_worldgen`
(the draw equals level 0's solid Structure, Plot, foundation, ground-cell and
bridge Voxels over a box; paths equal level 0's road and trail tops; no bridge
without spans), core `lod` (the writers), `server::lod::tests::structures`
(a hollow tower's Impostor occupies exactly the coarse Voxels its level-0
Voxels reach at levels 1 to 3; the placement generator's draw equals level 0's
per Chunk; a removed tower is absent at every coarse level and a placed Voxel
stands at every coarse level once saved and evicted in a persistent World;
built Materials leave the ground out). The old test that coarse levels hold no
Structure is replaced.

Agreement on Terra (01's harness, now with a Settlement window: the one Terra
spawns in, Chunk columns [47, -27] + 8). Ground stays exact at levels 1 to 5
over coast and forest, read without Impostors. Structures per coarse column,
against any level-0 column of its footprint standing a solid Structure Voxel
(`Worldgen::stood_at` tells a field's crops from Biome fields'); paths per
column centre where no Impostor stands:

| Level | columns | level 0 built in footprint | stood | missing | extra | path centres | top not path |
| ----- | ------- | -------------------------- | ----- | ------- | ----- | ------------ | ------------ |
| 1     | 16384   | 2.9%                       | 2.9%  | 0.00%   | 0.01% | 96           | 0            |
| 2     | 4096    | 4.0%                       | 4.0%  | 0.00%   | 0.02% | 16           | 0            |
| 3     | 1024    | 6.8%                       | 6.9%  | 0.00%   | 0.10% | 11           | 0            |
| 4     | 256     | 12.5%                      | 12.5% | 0.00%   | 0.00% | 0            | 0            |
| 5     | 64      | 23.4%                      | 23.4% | 0.00%   | 0.00% | 0            | 0            |

The test asserts no column misses a Structure, at most 1% stand one level 0
does not, and every path centre shows the path.

Coarse-Chunk CPU time. Thread CPU per Chunk, median of 9 interleaved rounds in
mirrored order, cold caches every pass (fresh generators, terrain regions
forgotten), Terra at its declared 160 m / 3000 m, a 3 x 3 Chunk-column block
over the Settlement window's middle, every row of the terrain band
(`agreement::coarse_chunk_cpu_time_with_and_without_structure_impostors_over_terra_settlement`,
`--ignored`, `optimized + debuginfo`). "Without" is the integration tip's job
(`58520661`) for an untouched coord, the same code with `structures: None`;
"with" adds the Structure draw and paths. Host load 7.7-9.1:

| Level | Chunks | without ms | with ms | with / without |
| ----- | ------ | ---------- | ------- | -------------- |
| 1     | 63     | 10.69      | 10.71   | 1.00           |
| 2     | 36     | 4.55       | 5.52    | 1.21           |
| 3     | 27     | 2.64       | 5.63    | 2.13           |
| 4     | 18     | 4.64       | 15.27   | 3.29           |
| 5     | 18     | 8.11       | 31.39   | 3.87           |

- With bridges at every level the same run read 21, 81, 445 and 1,518 ms at
  levels 2 to 5 (host load 20-30): the gap search, 97% of the draw at levels 3
  and 5 by a sampled profile, though no bridge stands in this window.
- Level 1 pays nothing: its draws read the 1 m columns its source has warmed.
- This window is Terra's densest: a town and a village. Levels 4 and 5 pay 11
  and 23 ms there, once per Chunk column and row, for a ring of few Chunks;
  most Chunks hold no Structure and pay only the claims lookup. Caching the
  claims per coarse Chunk column across rows would halve 4 and 5; the
  whole-field fill time is 09's series.

Captures (not committed, for the review batch after 05):
`/tmp/lod22-04-structures-cD3O`. Eye east of the spawn Settlement looking west
(yaw 90), `before-*` from the integration tip (`58520661`), `after-*` from this
branch, `dev` build, offscreen 1920 x 1080, no flight:
`settlement-0.5km` (Eye 2154.5, 95.5, -759.5, pitch -6.3),
`settlement-1km` (2654.5, 135.5, -759.5, pitch -5.4) and `settlement-2km`
(3654.5, 215.5, -759.5, pitch -5.0); `-settled` waited 30 s after the fill,
the others 8 s. At 1 and 2 km the village and its dirt paths stand where the
tip shows bare ground; at 0.5 km a forest at level 1 hides it, and only
nearer Plot Structures differ.

Ran: `cargo fmt --all`, `npm run fmt`, `cargo test -p tesserae-core` (1,073
pass), `cargo test -p tesserae --bins` (1,686 pass, 5 ignored), `cargo clippy
-p tesserae -p tesserae-core --all-targets --all-features`.
