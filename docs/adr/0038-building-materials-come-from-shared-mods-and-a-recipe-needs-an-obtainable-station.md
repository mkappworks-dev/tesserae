# Building Materials come from shared Mods, and a Recipe needs an Obtainable Station

Status: accepted.

Every demo Game declared its own building Materials, Stations, chest, Door,
glass and lamp, each with its own Recipe, while the shared `timber` and
`brickwork` Mods stood unused beside them and duplicated three of Terra's
outright. A Mod author had nothing shared to build a house from that would fit a
Game's village, and each new Game declared the whole set again.

## Building Materials are shared Mods' content

The Materials a Structure is built of, and the Stations they are made at, are
declared by shared Data-only Mods: `workshop` (the `workbench`, `forge`, `anvil`
and `oven` Stations and the chest), `timber` (planks, frames, roofing, the Door
and the gate), `brickwork` (masonry, tile and whitewash) and `glassworks` (glass
and the lantern). Each carries its own Items and Recipes, made from shared world
Materials (`flora`'s logs and leaves, `crops`' wheat, `strata`'s stone, clay and
sand) at `workshop`'s Stations, so a Template that names them is Obtainable in
any World whose Game lays those.

Terra builds from them alone, and makes strata's stone and sand from its own
rock and sand by hand, since strata lies only in its bedding. The other demo
Games take the shared Stations and furnishing and keep their themed building
Materials, which are their look and already have Recipes of their own; a shared
Material retinted per Game would need a Game to supply appearance for another
Mod's Material, which the contract does not allow, and is deferred. So is a
Recipe input by tag (`#stone`): two bridge Recipes cover Terra.

A Station's tag is a global name, so a Game's own themed bench under the tag
`workbench` opens the same Recipes as `workshop`'s.

Nothing is migrated: a World made before refuses to open, through the save's
refusal of a missing Material and of a changed Mod build, as
[ADR-0033](0033-generator-version-3-is-the-only-version.md) did before the first
release.

## A Recipe counts towards Obtainable only at an Obtainable Station

[ADR-0029](0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md)'s
amendment left Recipe Stations unchecked, so a Material made only at an oven no
Player could get passed. Now a Recipe whose station is not `hand` counts only
once a Material declaring a Station with its tag is itself Obtainable; when that
is all that fails, the refusal names the tag.

## Every declared Template is checked

The check covers every Template any loaded Mod declares and every declared
Settlement's path, not only those a Biome stands. A Template only the Creative
Tool places is still a Structure, and a Site (step 23) will build one from
Items. The exemptions stand: `ground` cells, fluid cells and a crop's later
Stage.
