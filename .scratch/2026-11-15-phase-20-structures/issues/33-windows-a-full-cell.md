# 33: Windows are at least a full cell of glass

Status: done

Blocked by: 22

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found the Templates' glass windows not reaching their sill. A
storey's window is a half-slab sill (`trim`, row 1) under one cell of glass
(row 2): the slab fills only the lower half of its cell, so a 0.5 m gap stands
open between the sill and the glass.

- Every window is at least one whole cell of glass with no gap round it: the
  sill is a whole cell, or the glass comes down to meet it.
- Larger buildings vary their windows: some 1 wide by 2 tall, some 2 by 2, the
  small houses keep 1 by 1. Each kind keeps its own look and Materials.
- Shapes come from the mockup's builders (`mockup/structures.js`), exported by
  `node mockup/export-templates.mjs`.

## Acceptance criteria

- [x] No Template has a gap of air or a slab's open half between a window's
      glass and its sill, tested over every Game's Templates.
- [x] At least the inn, warehouse, barn and farmhouse have windows 2 tall, and
      some 2 by 2, in the mockup and every Game.
- [x] The walk, door, undo-budget and mockup export checks still pass.

## Comments

Mockup: `building()` takes `windows: [wide, tall]` in cells (1 by 1 unless
given) and lays glass from row 2 for `tall` rows wherever `along % (wide + 2)
>= 2`, before the base course and the timbers, on a whole `trim` cell in row
1 (it was a `trim` slab). Every Game's Templates re-exported.

Sizes: farmhouse and inn 2 by 2; family house, warehouse and barn 1 wide by 2
tall; cottage, bakery and the rest 1 by 1 (the towers' windows already sat on
whole cells).

Test: `every_window_sits_on_a_whole_cell_and_the_larger_buildings_have_tall_ones`
(`demo_games/rooms.rs`), over every Game's Templates: each window's lowest pane
stands on a whole cube, the farmhouse, inn, warehouse and barn have a window 2
tall, and some Settlement building one 2 by 2. Failed on the old sills.
