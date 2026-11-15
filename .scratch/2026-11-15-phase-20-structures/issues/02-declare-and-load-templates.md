# 02: A Template is declared and loads

Status: ready-for-agent

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

- [ ] A Mod with a Template loads, and the World can sample it at 1.0, 0.5 and
      0.25 m with the same footprint in metres.
- [ ] All four turns give the right Orientation for every Shape, and a quarter
      turn four times is the identity.
- [ ] A Keep cell and an Air cell are different cells.
- [ ] An unknown Material or Shape, a grid over 64 m a side, over 65,536 cells,
      a 65th Template, or a missing Ground layer is a load error naming the
      Template and cell, from the data path and a component alike.
- [ ] The 0.1 and 0.2 adapters give the list as empty, and a Declaration with no
      Template loads as before.
- [ ] Templates are named `<mod id>:<name>` and any Mod may declare one.
