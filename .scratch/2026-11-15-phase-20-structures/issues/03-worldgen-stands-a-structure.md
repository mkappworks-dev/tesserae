# 03: Worldgen stands a Structure

Status: done

Blocked by: 02

Spec: [Phase 20](../spec.md).

## What to build

A Biome lists `structures`: a Template, a spacing in metres and a chance, at
most 4 rows, with spacing no smaller than twice the Template's longer side.
Worldgen holds a Structure per lattice cell by a hash of the seed and the cell,
salted per row, with its rotation, inset so two never touch, and writes its
cells into the Chunks it reaches: every non-Keep cell, Air included, overwrites
what is there. The pass exists only when a Biome lists a Structure. This ticket
places on any ground; the ground rules come in ticket 04.

## Acceptance criteria

- [x] A Structure is whole across Chunk borders, equal in metres at 1.0, 0.5 and
      0.25 m, and the same whichever Chunk is generated first, with more than a
      few crossing a border.
- [x] All four turns appear and each Structure's footprint is the same in metres
      turned.
- [x] Keep cells leave the terrain untouched and Air cells carve it.
- [x] A World whose Biomes list no Structure generates the digests recorded
      before this ticket, and a version 1 World is unchanged.
- [x] A fifth row, an unknown Template or a spacing under its floor is a load
      error naming the Biome.
- [x] A Biome stays inline within its size limit with 4 rows.

## Comments

Where things live, for 04, 05, 06, 07, 13 and 14:

- Biome row: `tesserae_core::BiomeStructure { template: u16, spacing_m,
  chance }`, `template` an index into the World's Templates
  (`Tables::templates`, load order). Held inside `SurfaceFeatures`, like crop
  fields, so no `Biome` literal changed: `with_structures(&[..])` (at most
  `MAX_BIOME_STRUCTURES` = 4) and `structures()`. `BiomeStructure::validate(
  &Template)` checks chance in (0, 1] and spacing from twice the longer side
  to `MAX_STRUCTURE_SPACING_M` (4096 m). A Biome with 16 features, 2 fields
  and 4 rows is under 3 KiB.
- The generator takes Templates by `Worldgen::with_templates(&[Template])`,
  after `new`; the Server's `open_world` passes `ContentTables`'
  `templates`. Without it, or when no Biome lists a Template the slice has,
  there is no pass. LOD sources and every other `Worldgen::new` take none, so
  coarse levels already leave Structures out (07 still owns the test).
- The pass: `crates/tesserae-core/src/worldgen/structures.rs`.
  `StructurePass` (built in `with_templates`) holds the Templates in an `Arc`
  and the rows in claim order (Biome, then row) with each Template's sampled
  size per turn and Ground layer. `Worldgen::structures_near(region)` returns
  every `Structure` covering or bordering columns `[min, max)`;
  `Structure::footprint()` is its column rectangle after the turn, which is
  what 05 skips. `stand_structures` writes one Chunk's cells.
- Mods: WIT `biome-structure { template, spacing-m, chance }` and
  `structures` on `biome-def` before `soundscape`; TOML `structures = [{
  template, spacing-m, chance }]` on `[[biomes]]`. Resolved in
  `resolve_biomes` by `biome_structures` against `tables.templates`, so
  Templates must resolve before Biomes (they already do in `declare_all`).
  Errors are `ModError::BadStructure` and read "mod `t`: biome `plain`
  structure 2: ...". Every Game's `BiomeDef` gained `structures: Vec::new()`.

Decisions:

- A lattice cell is held by `hashed(seed, salt, [biome, row, i, j])` against
  `chance`; up to 4 attempts each draw a turn and a corner. The corner is on
  whole metres, inset 0.5 m from the cell's border, so a Structure stands at
  the same metre place and turn at 1.0, 0.5 and 0.25 m, and two in
  neighbouring cells are at least about 1 m apart.
- Rows of other spacings, other Biomes and the same spacing can overlap, so
  a later row's draw gives way to any earlier row's unfiltered Structure it
  would touch, as crop fields do. This is the claim order: Settlements (14)
  slot in as rows before every single Structure, checked the same way.
- `structure_accepts` is the one hook 04 replaces: for now it takes a draw if
  the footprint's centre is in the row's Biome, and stands the Ground layer's
  lowest Voxel layer on the top Voxel of the ground under that centre (the
  surface Voxel, so a floor is flush with the grass and `ground` in 13 is "at
  and above the Ground layer"). 04 adds every-metre Biome, lake, cave and
  tolerance checks there and swaps the centre for the median.
- The pass runs after smoothing and before scatter, plants and fields, and
  also on the two early-out paths (sky above the terrain band, deep rock) when
  `structures_may_reach` says a listed Template's height or cellar can reach
  them, so a tower above the band is whole. `surface_bounds_m` is unchanged:
  widening the Server's streaming and open-sky bounds is 07's.
- A Biome's centre column is read at the metre point of the footprint's
  centre, so the draw is decided alike at every Voxel size.

Deferred:

- The snapshot does not carry `structures` yet (06): a reopened World's
  Biomes have none, so new Chunks there stand none.
- Vegetation is not kept off yet (05). Trees and scatter write only into
  air (scatter only above the Biome's surface Material), so they can stand
  inside a Structure's Air cells but never replace its Voxels; a crop
  field's path and ditch are laid over the top Voxel and can replace a
  Structure's Ground layer.
