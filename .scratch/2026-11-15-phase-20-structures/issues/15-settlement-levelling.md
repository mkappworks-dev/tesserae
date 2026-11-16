# 15: Plots are levelled and roads graded

Status: done

Blocked by: 14

Spec: [Phase 20](../spec.md).

## What to build

Each Plot is cut and filled level under its footprint and a 2 m apron, by at
most 2 m, blended out to the natural ground. Roads are laid in the path Material
and become stairs and slabs where they climb. Voxels are edited in the pass; the
heightmap is unchanged.

## Acceptance criteria

- [x] Every Plot is level across its footprint, and no cut or fill exceeds 2 m.
- [x] The apron blends into the natural ground with no cliff at its edge, by a
      slope test.
- [x] A road never rises more than a slab per step and is walkable by the
      Player's step height.
- [x] Levelling is the same whichever Chunk is generated first.
- [x] Wilderness Structures keep the strict level rule and are never levelled.

## Comments

Tests: `crates/tesserae-core/tests/settlement_worldgen.rs`
(`every_plot_is_level_and_no_cut_or_fill_exceeds_two_metres`,
`the_apron_blends_into_the_ground_with_no_cliff_at_its_edge`,
`a_road_never_rises_more_than_a_slab_per_step`,
`wilderness_structures_keep_the_strict_rule_and_are_never_levelled`), on a
12 m relief World; order independence through `Laid::scan`, which scans the
Chunks in both orders.

Decisions:

- Plot rule (`plot_ground`): metre samples over the footprint and its 2 m
  apron; dry everywhere, no cave in the footprint's top metre, and every
  sample within 1.5 m of the lower median of the footprint's samples. 1.5,
  not 2, so a column between samples still keeps within 2 m; each column's
  target is also clamped to 2 m. A Template's own level tolerance is no
  longer asked of a Plot; single Structures keep it.
- Levelling: per column, the Chebyshev distance `d` from its centre to the
  footprint; the target is the Plot's height eased toward the column's own
  ground by `d / 3 m`, so the apron's outer columns are a third of the way
  back and the next column is the ground. The top Voxel moves to the target:
  soil fills beneath (the old surface Voxel becomes soil), the Biome's
  surface Material caps it, and air clears above through the old top and
  any smoothing stair. Plots stand with no foundation, since the ground is
  level under them.
- Roads: graded over the town's metre grid as the mean of the highest
  surface under the road's ground and the lowest over it, each no steeper
  than 0.45 rise over run (two-pass L1 distance transforms), so the grade is
  0.45-Lipschitz everywhere. A road cell whose grade is more than 2 m off
  its ground is not laid. Each column takes the bilinear grade at its
  centre; its walked height is in half Voxels, above the Voxel whose floor
  is at or under the grade, as the ground's own top is. An odd half is a
  slab in the path Material, or a stair facing the one road neighbour half a
  Voxel higher. Adjacent columns differ by at most half a Voxel at any
  size, since 0.45 of a Voxel is under one half Voxel step.
- The declared path Shape is used on whole-Voxel road; climbing uses the
  built-in slab and stair in the path Material.
- Order in a Chunk: every Plot's ground levelled, then roads, then Plot
  Structures, all from metres per column, so the same whichever Chunk is
  first. The heightmap (`surface_height_m`) is unchanged.
- `structure_band_m` widens by 2 m (plus a Voxel) wherever a Settlement is
  listed.

For later tickets:

- 18: levelling costs a surface sample per apron column per Chunk, and
  grading a pass over the disc's square (at most 129 by 129 cells) per
  layout.
- A road may meet an apron's outer edge up to 2 m off it, where the road is
  cut or filled; the apron eases to the natural ground, not the road.
