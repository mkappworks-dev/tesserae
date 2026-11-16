# 14: Worldgen lays out a Settlement

Status: done

Blocked by: 04, 05

Spec: [Phase 20](../spec.md).

## What to build

A Biome lists `settlements`: at most 2 rows naming a Settlement and a spacing.
A Settlement declares a radius of at most 64 m, an optional centre Template, up
to 24 Plots from weighted Templates with minimum and maximum counts, an overall
minimum, and a path Material and Shape. Settlements have their own salted
lattice and claim ground before single Structures, fields, trees and scatter,
with a 16 m margin that keeps single Structures out. Roads branch from the
centre on a slope-aware path that avoids water and over-steep ground; Plots grow
beside roads with each door facing the road. A Plot whose ground fails stays
empty, and the Settlement is dropped below its overall minimum. This ticket lays
roads on the natural ground; levelling is ticket 15.

## Acceptance criteria

- [x] A Settlement is whole across Chunk borders, the same whichever Chunk is
      generated first, and equal in metres at 1.0, 0.5 and 0.25 m.
- [x] Every Plot's door faces a road, no two Plots overlap, and counts stay
      within each Template's bounds.
- [x] No single Structure, field or tree stands in a Settlement's disc, and no
      single Structure within its margin.
- [x] A World whose Biomes list no Settlement generates its recorded digests.
- [x] A third row, a radius over 64 m, more than 24 Plots or an unknown Template
      is a load error naming the Settlement.
- [x] Settlements ride in the generation snapshot with the Templates they use.

## Comments

Tests: `crates/tesserae-core/tests/settlement_worldgen.rs` (through
`Worldgen::chunk`), `tests/settlement_snapshot.rs`, loader tests
`crates/tesserae-mods/src/tests/settlements.rs`, and the Server reopen test
in `server/persistence/tests.rs`.

Decisions:

- Core type `crates/tesserae-core/src/settlement.rs`: `Settlement { name,
  radius_m, margin_m, centre: Option<Building>, plots, rules: Vec<PlotRule>,
  minimum, path: Voxel }`, `Building { template, door: Option<[u16; 2]> }`.
  World table `Tables::settlements`; Biome row `BiomeSettlement { settlement,
  spacing_m }` via `SurfaceFeatures::with_settlements` / `settlements()`, at
  most 2. Spacing floor is twice radius plus margin, so a row's discs and
  margins never overlap. `Worldgen::with_settlements` after `with_templates`.
- Door side: `Template::door_cell(is_door)`, the first Door (closed) cell in
  the lowest layer from the Ground layer up; resolved at load from the Door
  voxel-use and frozen in the snapshot. The Template edge nearest that cell
  faces the road. No door: the +Z edge faces the road.
- Layout (`worldgen/settlements.rs`), all in metres: centre on whole metres
  in the lattice cell; the centre Template (if any) must pass the ground
  rules or the Site is refused. 3 or 4 roads branch from the centre in 4 m
  steps, each step the gentlest of 5 bends that stays in the disc, dry, off
  the centre Plot and at most 0.45 rise over run. Road cells are metre cells
  within 1.5 m of the line. Plots are tried every 2 m along each road, both
  sides, outward from the centre: the Template's door is placed 1.5 + 2 m
  back from the road point, the corner rounded to whole metres, the Plot plus
  a 2 m apron must lie in the disc, off every road cell and off other Plots.
- Draw order: each rule's minimum first, then weighted draws up to `plots`
  within each max. A Plot whose ground fails stays empty; a minimum draw
  retries at the next slot, an extra one is spent. Dropped when a minimum is
  unmet or fewer than `minimum` Plots stand.
- Plot ground (this ticket): as a single Structure's, the Template's level
  tolerance over metre samples, dry, no cave; Ground layer on the lower
  median. Roads replace each column's top Voxel with the path Voxel and clear
  a smoothing stair above. Ticket 15 replaces both.
- Claims: `Worldgen::claims_near(region)` computes the Settlements (full
  layouts) and single Structures once per Chunk; footprints for fields, trees
  and scatter include each Settlement's disc (`Footprint` now carries an
  optional disc). Single Structures keep out of disc plus margin of every
  accepted Site (centre passes), asked cheaply without a layout; so a Site
  later dropped below its minimum still keeps Structures out.
- Rows: a later row's Site gives way to an earlier row's whose disc and
  margin it overlaps. A Site's centre must be in a Biome listing it.
- `structure_band_m` widens by the tallest and deepest Template a listed
  Settlement uses. Spawn clearance counts Plots.
- Snapshot: `Generation.settlements`, `SavedBiome.settlements`; `capture`
  takes the Settlements table and keeps their Templates in `kept`.
  `Generation::settlements(mapping)`. Server: `LoadedMods.world_settlements`,
  `WorldParameters.2`.
- Declaration: WIT `settlement-def`, `settlement-plot`, `biome-settlement`;
  TOML `[[settlements]]` (weight 1, min 0, max 24 by default) and Biome
  `settlements`. Validator `tesserae_mods::DeclaredSettlements`; its path
  Material is checked Obtainable as Templates are.

For later tickets:

- 15: levelling replaces `plot_ground` and the road stand in
  `stand_town`; widen `structure_band_m` by the 2 m cut and fill.
- 18: each Chunk lays out every Settlement whose disc reaches it, including
  sky and deep Chunks in the band; a layout costs a few hundred surface
  samples. A per-generator memo of recent layouts is the lever if needed.
