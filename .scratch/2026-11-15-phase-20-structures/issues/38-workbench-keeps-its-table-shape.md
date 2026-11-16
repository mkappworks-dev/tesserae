# 38: A Template's workbench keeps its table Shape

Status: done

Blocked by: 22

Spec: [Phase 20](../spec.md).

## What to build

Play-testing: "the workbench style has change to a box instead of a bench".
Every Game's workbench row fixes its Shape to the Game's `table` (legs under a
top), so a Player's Place puts it as a bench; but the mockup's builders wrote
Template workbenches as plain cubes, so worldgen and the place Tool stood a
box.

- A Template cell of a Material whose use row fixes a Shape is in that Shape.

## Acceptance criteria

- [x] Every Game's Templates hold their workbenches in the Game's `table`
      Shape, tested over every Template cell whose Material fixes a Shape.
- [x] The mockup export check passes.

## Comments

Mockup: `FIXED_SHAPE` (`structures.js`) gives `M()` a role's fixed Shape by
default, `bench` → `table`; every Game's Templates re-exported (five
workbench rows per Game, now `<game>:table`). Doors were already in their
`panel`. Test:
`every_template_cell_of_a_material_that_fixes_its_shape_is_in_that_shape`
(`demo_games/structures.rs`); it failed on the cottage's workbench before.
