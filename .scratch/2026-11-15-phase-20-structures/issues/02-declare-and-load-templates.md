# 02: A Template is declared and loads

Status: done

Blocked by: None

Spec: [Phase 20](../spec.md).

## What to build

A Game or Mod declares a Template in TOML, or a component returns one, and the
World loads it. The core gains the Template type: a grid in metres, a palette of
Material, Shape and Orientation by name, layers from the bottom up, Keep and
Air cells, a Ground layer, an optional foundation Material and a level
tolerance. It samples a Template at a World's Voxel size by nearest cell and
turns it in quarter turns, composing each cell's Orientation. The Declaration
gains a `templates` list after Plants on the unreleased 0.3 line, and the
contract-compatibility document lists it under "Amended before release".

## Acceptance criteria

- [x] A Mod with a Template loads, and the World can sample it at 1.0, 0.5 and
      0.25 m with the same footprint in metres.
- [x] All four turns give the right Orientation for every Shape, and a quarter
      turn four times is the identity.
- [x] A Keep cell and an Air cell are different cells.
- [x] An unknown Material or Shape, a grid over 64 m a side, over 65,536 cells,
      a 65th Template, or a missing Ground layer is a load error naming the
      Template and cell, from the data path and a component alike.
- [x] The 0.1 and 0.2 adapters give the list as empty, and a Declaration with no
      Template loads as before.
- [x] Templates are named `<mod id>:<name>` and any Mod may declare one.

## Comments

Where things live, for 03, 08, 12 and 13:

- Core type: `tesserae_core::template` (`crates/tesserae-core/src/template.rs`).
  `Template::new(name, cell_m, layers, ground_layer, foundation,
  level_tolerance_m)` checks the grid; `Template::sample(voxel_size, Turn)`
  gives a `Sampled` with `size()` (Voxels after the turn), `ground()` (lowest
  Voxel layer of the Ground layer) and `cell([x, y, z])`, a `Cell` of `Keep`,
  `Air` or `Voxel` with its Orientation already turned. `Turn` is a quarter
  count; one quarter takes the Template's +Z edge to +X. Limits are
  `MAX_TEMPLATE_SIDE_M`, `MAX_TEMPLATE_CELLS`, `MAX_TEMPLATES`.
- `Orientation::yawed(quarters)` in `shape.rs` turns an Orientation about the
  World's +Y whatever its `up`; `turned` stays a turn about the Shape's own up.
- The World's Templates are `Tables::templates`, a `Vec<Template>` in load
  order, names `<mod id>:<name>`. Look one up by name there.
- The one validator: `tesserae_mods::DeclaredTemplates`
  (`crates/tesserae-mods/src/template.rs`), taken from the Behaviours before
  `resolve` and resolved after it; the Server does so in `declare_all`
  (`server/mods.rs`) after Plants. Errors are `ModError::Template` and read
  "mod `t`: template 0 `hut`: ...". The TOML side is
  `crates/tesserae-mods/src/data/template.rs`.
- WIT: `template-def`, `template-cell`, `template-voxel` and
  `template-palette-entry` in `wit/0.3/content.wit`, and `templates` appended
  last on `declaration` (after `death-marker-kind`, which follows Plants). The
  starter's copy of `content.wit` is synced. Listed in
  `docs/contract-compatibility.md`.

Decisions:

- Sampling takes, along each axis, the Voxels whose centre falls inside the
  side, each in the cell its centre is in (with a 1e-3 guard). A side that is
  not a whole number of the World's Voxels rounds to the nearest; a 0.5 m side
  in a 1 m World is one Voxel. The footprint is equal at 1.0, 0.5 and 0.25 m
  when each side is a whole metre.
- `ground-layer` is optional in the record and required by the validator, so a
  missing one is the same error from data and a component.
- A palette Material may be any non-fluid Material (a flower is fine); a fluid
  is refused, since cells carry no occupants. A cube cell's Orientation turns
  too, harmlessly.
- A palette error names the Template and the palette character; a cell not in
  the palette names its layer, row and column. Rows run +Z, characters +X.
- The 65-Template limit is per World, checked in load order, so the error names
  the 65th Template and its Mod. There is no per-Mod limit.

Left for later tickets: the `ground` palette word (13; the TOML palette value
is a string word or a table, so it slots in beside `keep` and `air`), the
Grown-Material and Door-open refusals and Obtainable (12), Biome rows (03).
