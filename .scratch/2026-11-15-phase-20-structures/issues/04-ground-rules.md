# 04: Structures stand only on suitable ground

Status: done

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

A lattice draw is accepted only where every sampled footprint column, a metre
apart, is in a Biome that lists the Template, the ground varies by at most the
Template's tolerance, no lake covers a column and no cave cuts a column's
surface. Up to 4 draws are tried per lattice cell and the first that passes
wins. The Template's Ground layer sits at the footprint's median ground height,
layers below it overwrite underground, and the foundation Material fills any
gap beneath the lowest layer down to ground.

## Acceptance criteria

- [x] No accepted Structure stands in a lake, over a cave mouth, on ground
      steeper than its tolerance or in a Biome that does not list it.
- [x] A Template with a cellar has its cellar underground and its Ground layer
      level with the surface.
- [x] A foundation Material fills the gap on uneven ground and is absent when
      none is declared.
- [x] Over caves and smoothed ground, a Structure is still whole across Chunk
      borders and equal at all three Voxel sizes.
- [x] A rejected draw leaves the cell empty or takes a later attempt, the same
      whichever Chunk is generated first.

## Comments

Tests: `crates/tesserae-core/tests/structure_worldgen.rs`, through
`Worldgen::chunk` (`no_structure_stands_in_a_lake`, `..._over_a_cave_mouth`,
`..._on_ground_steeper_than_its_tolerance`, `..._only_in_a_biome_that_lists_it`,
`a_cellar_is_underground_...`, `a_foundation_fills_the_gap_...`,
`over_caves_and_smoothed_ground_...`). The test `Probe` now compares a World
against the same World with no rows, so lakes and caves show in both.

Decisions:

- Every rule asks metres only, so a draw is accepted alike at 1.0, 0.5 and
  0.25 m: columns from the footprint's whole-metre corner, a metre apart
  (`SAMPLE_M`); the continuous surface height, not a Voxel's top.
- Biome: each sampled column's Biome must list the Template (any row), not
  only the drawing row's Biome, so a Structure may cross into a Biome that
  also lists it (13 relies on this). `StructurePass::listed` holds a bit per
  Template per Biome; a World has at most 64 Templates.
- Lake: refused where a sampled column's surface is under the lake level,
  which is where some Voxel size floods it.
- Caves: `CaveField::may_carve` over the metre under a sampled column's
  surface. `false` is a proof for every Voxel size's surface Voxel, so the
  rule is conservative near tunnels and equal across sizes; it runs last, as
  the dearest ask.
- Tolerance: highest minus lowest sampled height, against the Template's
  `level_tolerance_m`, failing as soon as it is exceeded.
- Height: the Ground layer's lowest Voxel layer replaces the top Voxel of the
  lower median of the sampled heights. A column one Voxel lower than that
  leaves no gap (the floor sits in the surface Voxel), so a foundation shows
  only where ground falls two Voxels or more below.
- Foundation: under each column whose lowest-layer cell is a Voxel (not Keep
  or Air), from the column's own top Voxel up to the Structure's lowest
  layer. Computed per column from the surface, so it is whole across Chunks;
  it overwrites smoothing stairs and lake water in that gap.

For later tickets:

- Only sampled columns are ruled on: at 0.5 and 0.25 m a column between two
  samples can sit a little lower than the tolerance says, or in shallow lake
  water at the shore.
- The rules cost a surface sample per square metre of footprint plus a cave
  range test, per draw and per Chunk asking; 18 measures it.
