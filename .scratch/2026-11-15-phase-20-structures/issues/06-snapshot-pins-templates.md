# 06: A World keeps the Templates it was made with

Status: done

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

A World freezes the Templates its Biomes reference in the generation snapshot,
by Material and Shape name, in a World-level table the Biome rows index into. It
is optional on read and absent when empty.

## Acceptance criteria

- [x] A saved World regenerates its unedited Structures unchanged after the
      Game's Template is edited.
- [x] A World saved before Structures loads and generates none, and no format or
      Generator version changes.
- [x] The snapshot round trips by name, and a snapshot naming a Material that is
      gone fails loading as any missing name does.

## Comments

Where things live, for 13, 14 and later:

- Snapshot: `Generation.templates: Option<Vec<SavedTemplate>>` in
  `crates/tesserae-core/src/save.rs`, absent when no Biome lists a
  Structure; `SavedBiome.structures: Vec<SavedStructure>` (absent when
  empty), `template` an index into that table, not the Game's. No format or
  Generator version change.
- `SavedTemplate` holds name, `cell_m`, size, ground layer, foundation (by
  name), tolerance, and cells as runs over a palette of `keep`, `air` or a
  `voxel` `Solid` (Material, Shape, Orientation by name), in `Template::cell`
  order (x, then z, then y).
- API: `Generation::capture(params, templates, mapping)` now takes the
  Game's Template table; `Generation::templates(mapping)` returns the
  World's, which `restore`'s Biome rows index into. `Commit::compatible`
  restores both, so a missing Material or Shape name fails with the same
  `Incompatible("missing Material ...")` as any other.
- Server: `LoadedMods::world_templates` is `Some` for a reopened World
  (set in `Persistence::prepare`); `WorldParameters` gained a second field,
  the Templates `open_world` passes to `with_templates` (the snapshot's, or
  the Game's for a new World). `ContentTables::templates` stays the Game's
  table, for anything that places a Game Template by name.

Decisions:

- Only the Templates a Biome lists are kept, renumbered in Game order, so a
  Template no Biome names is not frozen. Ticket 14 must add the Templates
  its Settlements name to the same kept set in `capture` (the `kept` list)
  and keep its own rows indexing the snapshot table.
- `restore` keeps its signature; Templates come from a separate
  `templates(mapping)` call, which also checks each row against its
  Template.

Tests: `crates/tesserae-core/tests/structure_snapshot.rs` (round trip
through JSON generating the same Chunks, edited Template not reshaping the
World, pre-Structures snapshot, missing Material); persistence test
`a_reopened_world_stands_the_templates_it_was_saved_with_whatever_the_game_declares_now`.
