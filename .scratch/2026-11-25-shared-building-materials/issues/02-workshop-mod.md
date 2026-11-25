# 02: The `workshop` Mod declares the shared Stations and chest

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Shared building Materials](../spec.md).

## What to build

A new Data-only Mod, `workshop`, depending on `flora` and `strata`. It declares
the `workbench`, `forge`, `anvil` and `oven` Stations, each a Material with a
Station voxel-use under that tag, the `chest` Container, and the `table` Shape
the workbench fixes. Each has an Item and the Recipe in the spec's table:
workbench from logs by hand, forge from stone by hand, oven from stone and sand
by hand, anvil from stone at the forge, chest from logs at the workbench. The
forge emits light as Terra's does. Appearance is colour only until the art
ticket lands. A README lists its Recipes and "Used by: nothing yet", and it has
a thumbnail like every shared Mod.

A small data-only fixture Game, used by the Server's Obtainable tests, lays
`strata:stone`, `strata:sand` and `strata:clay` and stands a flora tree and a
wheat field, so a shared Mod's Materials can be tested as Obtainable without
Terra. Ticket 03 reuses it.

## Acceptance criteria

- [ ] `workshop` composes with only its `depends` and passes validate.
- [ ] Under the fixture Game, each of the five Materials is Obtainable.
- [ ] Each Station opens the Recipes that name its tag; a second Mod's Station
      with the same tag opens the same Recipes.
- [ ] The chest holds Stacks and spills them when removed by anything but a
      Player, as a Container does.
- [ ] The workbench is drawn and collides as the `table` Shape.
- [ ] The shared-Mods inventory test lists `workshop`, and its thumbnail exists.
