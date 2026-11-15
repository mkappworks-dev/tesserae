# 13: A Template takes the local ground

Status: done

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

A palette entry may be `ground`. Where a Structure stands, it is the Biome's
surface Material at and above the Ground layer and its soil below, so one
Template fits grass, sand or snow.

## Acceptance criteria

- [x] The same Template stands on grass in a meadow Biome and on sand in a
      desert Biome, by test.
- [x] A Structure across a Biome border takes each column's own Biome.
- [x] `ground` passes the Obtainable check by definition.

## Comments

Where it lives:

- Core: `Cell::Ground` in `tesserae-core/src/template.rs`;
  `Sampled::ground_voxel(y, GroundParams)` gives the soil below the Ground
  layer's lowest Voxel layer and the surface Material at and above it, as a
  cube. `Worldgen::column_ground(gx, gz)` is the ground of a column at global
  Voxel indices, as the Chunk holding it lays it (the Biome's, else the
  World's; `None` without ground).
- Worldgen: `stand_structures` resolves a `ground` cell per column, once per
  column. A `ground` cell in the lowest layer counts as solid for the
  foundation (04).
- Server (08's `Placement`): takes the `WorldGenerator` and resolves a
  `ground` cell the same way, so a placed Structure fits the Biome it lands
  in. With no generator or no ground it is left like Keep.
- Mods: TOML palette word `"ground"` beside `"keep"` and `"air"`; WIT
  `template-cell` gains `ground` (0.3, amended before release; starter WIT
  synced, guests rebuilt).
- Obtainable (12): `DeclaredTemplates::check` only looks at Voxel cells, so
  `ground` passes by definition; the Biome's own ground is laid by worldgen
  anyway. Test `a_ground_cell_is_obtainable_by_definition` in
  `server/mods.rs`.

Tests:

- core, through `Worldgen::chunk` at 1.0 and 0.5 m, whole Structures in each
  Biome and across a border, every `ground` cell as its own column's Biome
  lays it:
  `a_ground_cell_takes_grass_in_a_meadow_sand_in_a_desert_and_each_columns_own_across_a_border`
- mods: `a_ground_cell_loads_from_data_and_a_component`
- Server placement:
  `a_ground_cell_takes_the_soil_below_the_ground_layer_and_the_surface_above`

Decisions:

- `ground` is a whole cube: no Shape or Orientation, so a slab of grass is
  not expressible; a later ticket can add `{ ground = true, shape = ... }`.
- Rock is never used: below the Ground layer is soil however deep the
  cellar goes.
- For 10/11 (aim answer and ghost): a `ground` cell's Voxel depends on the
  column, so the aim answer must resolve it through `column_ground` too.
