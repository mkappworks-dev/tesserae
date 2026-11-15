# Structures are declared Templates placed as pure functions of metres

Status: accepted.

Step 20 lets a Game or Mod declare a **Template**, a named block of Voxels, and
lets world generation and a Creative Player's Tool place it as a **Structure**.
Nothing is stored per Structure: a placed one is only Voxels, so saving,
meshing, light and collision need nothing new, and no Generator version moves
([ADR-0022](0022-biomes-are-column-content-and-a-world-pins-its-generator.md)).

## A Template is data in metres, named by Material

A Template is one more appended list in the unreleased 0.3 Declaration, in TOML
beside the Manifest
([ADR-0020](0020-data-only-mods-and-development-restarts.md)). Its cells name a
Material, a Shape and an Orientation by full `<mod id>:<name>`, never by id, as
the save does. The grid's cell size is in metres, and a World samples it at its
own Voxel size ([ADR-0003](0003-voxel-size-is-per-world.md)). Authoring in
Voxels was rejected: it would pin a Template to one Voxel size, or refuse every
World that differs.

## Worldgen placement is a lattice of pure draws

A Biome lists `structures` rows outside the 16 surface-feature slots. A square
lattice of `spacing-m` cells holds a Structure by a hash of the seed and the
cell, with its rotation, inset so two never touch, and tried up to 4 times
against the ground rules, as crop fields are. Each Chunk decides every Structure
that can reach it and writes only its own Voxels, so one is whole across Chunk
borders and equal in metres at every size. The pass reads no Chunk, so a
player's edit never changes what a neighbouring Chunk generates.

A Structure's non-`keep` cells always overwrite terrain, ore and lake water. A
Biome with no `structures` runs no pass. Trees, scatter and fields skip a
Structure's footprint, and a Structure never asks about them, which avoids a
cycle. Coarse levels leave Structures out, as they leave Plants out.

## Every Material is Obtainable

Each Material a Template names must be Obtainable in the World: reachable from
what the Biomes lay, grow or drop, through Recipes, with Structures and the
Catalogue not counted. It is checked when a World's Game and Mods are composed,
and a breach refuses the World, because a building must never be the only source
of its own Materials. A `ground` palette entry takes the local Biome's surface
or soil.

## Settlements are rules, laid out along roads

A Settlement is declared as rules (a radius, a centre, weighted Templates with
counts, a path Material), not drawn as one big Template, so no two are alike and
each follows the land. Roads branch from the centre on a slope-aware path; Plots
grow beside them with their doors facing the road, as Veloren lays out its
towns; pieces joined by connectors were rejected because they need connector
authoring and still do not fit plots to roads. Settlements claim ground first,
then single Structures, fields, trees and scatter, and nothing asks back up the
chain. Plots are cut and filled level in the generation pass, blended into the
ground around them, without changing the heightmap.

## A Player's Tool places, and never Uses to apply

The `place` Tool's Use only opens the Template list and answers; its Place verb
sends a separate Structure intent that the Server expands, as it does a Layout.
Placement is Creative-only, free, all-or-nothing and one undo group, so the
journal's cap counts cells rather than entries.

## Not decided here

Blueprints and Sites, which build a Structure over time from Items (step 20b),
and loot, entities, fluids, weathering variants and mirroring. The format
reserves room for them, and each needs Server machinery that belongs to a later
step. Existing trees stay parametric
([ADR-0027](0027-plants-are-declared-kinds-the-server-grows.md)).
