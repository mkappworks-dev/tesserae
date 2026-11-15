Status: ready-for-agent

# Phase 20: Structures

DESIGN.md roadmap step 20: a Template format, and deterministic placement by
world generation and by Tools. Depends on step 13 (Biomes, done). The Engine
adds the mechanism; a Game or Mod supplies Templates.

Vocabulary: **Template**, **Structure**, **Keep**, **Ground layer**,
**Obtainable**, **Settlement** and **Plot** are new and are in [GLOSSARY.md](../../GLOSSARY.md). The decision record is
[ADR-0028](../../docs/adr/0028-structures-are-declared-templates-placed-as-pure-functions-of-metres.md),
with an amendment to
[ADR-0003](../../docs/adr/0003-voxel-size-is-per-world.md). Related decisions:
[ADR-0001](../../docs/adr/0001-wasm-mod-boundary.md),
[ADR-0016](../../docs/adr/0016-only-air-is-built-in.md),
[ADR-0020](../../docs/adr/0020-data-only-mods-and-development-restarts.md),
[ADR-0022](../../docs/adr/0022-biomes-are-column-content-and-a-world-pins-its-generator.md),
[ADR-0023](../../docs/adr/0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md),
[ADR-0024](../../docs/adr/0024-game-mode-is-per-player-with-a-world-default.md),
[ADR-0026](../../docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md)
and
[ADR-0027](../../docs/adr/0027-plants-are-declared-kinds-the-server-grows.md).

## Problem Statement

A World's built things are all hand-placed. Worldgen can scatter a Material, a
tree or a crop field, but it cannot stand a cabin, a well or a ruin on the
ground, so a Survival player explores terrain with nothing made in it. A Game
author has no way to declare a building: the contract has no row for one, and
per [ADR-0001](../../docs/adr/0001-wasm-mod-boundary.md) the answer cannot be a
Mod calling the host once per Voxel. A Creative player has no way to put down
more than a line, rectangle or box of one Material, so a finished building means
placing it Voxel by Voxel.

A building authored for a 1 m World would also not survive a World at 0.5 m or
0.25 m if it were a literal grid of Voxels.

## Solution

A Game or Mod declares **Templates** as data. A Template is a grid of cells in
metres, each naming a Material, a Shape and an Orientation, or Keep. A Biome
lists the Templates worldgen may stand as **Structures**, with a spacing and a
chance, and a Creative Player holds the `place` Tool to stamp one wherever they
aim.

- A Structure has the same footprint in metres at every Voxel size, and is whole
  across Chunk borders whichever Chunk generates first.
- Worldgen picks where Structures go, and how each is turned, from the World
  seed alone, and accepts a place only on level, dry, un-cut ground in a Biome
  that lists the Template. Trees, scatter and crop fields keep off a Structure.
- A Structure's cells overwrite terrain, so a cellar is cut into the ground and
  a room into a hillside. A Keep cell leaves the ground as it is.
- The `place` Tool shows a ghost of the Structure with each cell tinted by what
  placing it would do, turns by quarter turns, and places the whole Structure or
  nothing, as one undo group.
- Nothing is stored per Structure. A World saved with Structures keeps them as
  Voxels, and a World saved before Structures generates none.
- Every Material a Template uses is Obtainable in the World: worldgen lays,
  grows or drops it, or a Recipe makes it from things that are. A building is
  never the only source of its own Materials.
- A palette entry may be `ground`, the surface or soil Material of the Biome a
  Structure stands in, so one house fits grass, sand or snow.

A Biome also lists **Settlements**: villages and towns that worldgen lays out
around a centre. Roads branch from the centre and follow the ground; Plots grow
beside them, each building turned so its door faces the road; each Plot is cut
and filled level, blended into the land around it, and a steep road becomes
stairs and slabs. No two Settlements are alike, and the same seed always gives
the same one.

Each of the five Games declares fifteen Templates, ten Settlement buildings
(homes, an inn, a warehouse, a blacksmith, a bakery, a market stall, a barn and
a well) and five wilderness Structures, and at least one Settlement; the SDK
starter declares one Template as the documented example. The Engine ships none.

Blueprints, Items that lay a Site which builds itself over time from Items in
the backpack or a chest, are step 20b, with its own spec.

## User Stories

Authoring

1. As a Game author, I want to declare a Template as data in TOML, so that I add
   a building without writing a Mod.
2. As a Game author, I want a Template's cells to name Materials and Shapes by
   name, so that it survives a change of Material ids.
3. As a Game author, I want to give a Template's grid size in metres, so that it
   looks the same in a World at any Voxel size.
4. As a Game author, I want a cell that is Keep, so that a Structure's ground
   shows through where I did not draw.
5. As a Game author, I want an Air cell that carves, so that I can cut a room
   into a hillside.
6. As a Game author, I want to name which layer sits at ground height, so that
   a Template can have a cellar and still stand level on the surface.
7. As a Game author, I want a foundation Material, so that a Structure on
   slightly uneven ground does not float.
8. As a Game author, I want a Shape and an Orientation on any cell, so that a
   Template can have stairs, slabs, fences and posts the way a Player builds
   them.
9. As a Game author, I want a Template to hold a Door's closed form, so that a
   Structure's doors work the way a hand-placed one does.
10. As a Game author, I want a Template to hold a Container, so that a chest is
    part of a building, empty until a Player fills it.
11. As a Game author, I want Templates from any Mod, so that a shared Mod can
    ship buildings every Game uses.
12. As a Mod author, I want a load error that names the Template and cell for an
    unknown Material or Shape, so that I find the mistake.
13. As a Mod author, I want a Template over 64 m a side, or over 65,536 cells,
    refused at load, so that a World never reads an unbounded Template.
14. As a Mod author, I want a Template to work unchanged on contract 0.3, so
    that I need no new line.

World generation

15. As a Game author, I want a Biome to list Structures with a Template, a
    spacing in metres and a chance, so that a Biome's buildings differ from
    another's.
16. As a Game author, I want at most 4 Structure rows per Biome, so that a Biome
    stays small and copyable.
17. As a Game author, I want a spacing no smaller than twice the Template's
    longer side, so that a Structure always fits its own cell.
18. As a Survival player, I want to find buildings as I explore, so that the
    World has something made in it.
19. As a player, I want Structures never to stand in a lake, so that a cabin is
    not half under water.
20. As a player, I want Structures only on ground that is level to within the
    Template's tolerance, so that they do not hang off a cliff.
21. As a player, I want Structures never to sit over a cave mouth, so that a
    floor is not a hole.
22. As a player, I want a Structure's rotation to vary, so that two cabins do
    not look alike.
23. As a player, I want trees and crop fields not to grow through a Structure,
    so that a house is not full of leaves.
24. As a player, I want a Structure whole across a Chunk border, so that I never
    walk into half a building at a seam.
25. As a player, I want the same Structure in the same place when I reload a
    World, so that a place I remember is still there.
26. As a player, I want a Structure I broke into to stay as I left it, so that a
    ruin I made stays a ruin.
27. As a player, I want never to spawn inside a Structure, so that I never begin
    stuck in a wall.
28. As a Game author, I want a World's Structures to stay as they were when I
    later edit a Template, so that a saved World is not reshaped under a player.
29. As a World's owner, I want a World with no Structures to generate exactly
    what it did before, so that an old World is untouched.

Placing with the Tool

30. As a Creative player, I want a `place` Tool, so that I can put down a whole
    building.
31. As a Creative player, I want a list of the Game's Templates, so that I pick
    which to place.
32. As a Creative player, I want to turn a Structure in quarter turns, so that
    it faces the way I want.
33. As a Creative player, I want a ghost of the Structure where I aim, so that I
    see what I am about to place.
34. As a Creative player, I want each ghost cell tinted by what placing it does,
    so that I see what it would replace.
35. As a Creative player, I want only the anchor to need to be in reach, so that
    I can place a building bigger than my arm.
36. As a Creative player, I want a placement to land whole or not at all, so
    that I never get half a house.
37. As a Creative player, I want a placement refused with a reason, so that I
    know why nothing happened.
38. As a Creative player, I want a placement to wait for Chunks still loading,
    so that I can place at the edge of what is loaded.
39. As a Creative player, I want one undo to remove a whole Structure, and one
    redo to put it back, so that a mistake costs one key.
40. As a Creative player, I want placing to cost no Items, so that a building
    does not need a pile of every Material.
41. As a Survival player, I want the `place` Tool refused, so that I cannot
    conjure a building.
42. As a player, I want a Structure I placed to be saved like any build, so
    that it is there on reload.
43. As a Mod author, I want to hear a placed Structure's Voxels as Edit events,
    so that a Mod reacting to Edits sees them.
44. As a player on a World at 0.25 m, I want a Structure to place quickly, so
    that a big building does not freeze the game.

Authoring tools and content

45. As a Game author, I want each Game to ship fifteen Templates and a
    Settlement, so that I have working examples of homes, trades and ruins.
46. As an SDK user, I want one Template in the starter, so that I see the format
    working.
47. As a Game author, I want the Engine to ship no Template, so that a World
    with no Game has none.
48. As a Game author, I want documentation of the format and the Biome row, so
    that I do not read the code.

Obtainable Materials and local ground

49. As a Survival player, I want every Material in a building to be one I can
    get, so that I can repair or copy what I find.
50. As a Mod author, I want a refusal naming the Template, the cell and the
    Material when a Material is not Obtainable, so that I add a Recipe or a
    dependency.
51. As a Game author, I want a `ground` palette entry, so that one Template
    takes the local grass, sand or snow.
52. As a Mod author, I want a Grown Material or a Door's open form in a Template
    refused, so that every cell has an Item.

Settlements

53. As a Survival player, I want to find villages and towns, so that the World
    has places people built together.
54. As a Game author, I want to declare a Settlement as rules (a radius, a
    centre, weighted Templates with counts and a path Material), so that every
    village differs.
55. As a player, I want roads between a Settlement's buildings, so that it
    reads as a place.
56. As a player, I want every house's door to face a road, so that a village
    looks lived in.
57. As a player, I want a village on a hillside to sit on levelled Plots
    blended into the slope, so that it does not float or sink.
58. As a player, I want steep roads built as steps, so that I can walk them.
59. As a player, I want no lone cabin at the very edge of a village, so that the
    two never collide.
60. As a player, I want a Settlement whole across Chunk borders and the same on
    reload, so that a village I remember is still there.
61. As a Game author, I want a Settlement smaller on poor ground rather than
    missing, and dropped only below its minimum, so that most Biomes still have
    villages.

## Implementation Decisions

Seams. Two, both at the highest existing point:

- World generation, through the generator's Chunk entry point, as the Plants
  tests do: generate Chunks of a World with and without Structures and compare
  cells. Pure, headless and in the core crate.
- The Server's edit path, through the Structure intent and its answer, as the
  Layout tests do. Declaration loading is tested through the Mod loader.

Template format:

- One more list appended to the 0.3 Declaration, after Plants. No new contract
  line; the compatibility document lists it under "Amended before release". The
  0.1 and 0.2 adapters give it as empty.
- TOML beside the Manifest, kebab-case, unknown fields refused. A Template has a
  name, a grid cell size in metres, a palette mapping one character to a
  Material, a Shape and an Orientation, and layers of rows of palette characters
  from the bottom up. A character is also Keep or Air. A Template names its
  Ground layer, an optional foundation Material, and a level tolerance in
  metres, default 1.0.
- Names are full `<mod id>:<name>` and resolved after every Mod has declared. A
  Template is at most 64 m a side and 65,536 cells; a World holds at most 64
  Templates. All are load errors naming the Template.
- Both the data path and a component Mod's returned record go through one
  validator.
- A palette entry may be `ground`, resolved where the Structure stands to the
  Biome's surface Material at and above the Ground layer and its soil below.
- Every Material a Template names must be Obtainable in the World it is used
  in. Obtainable is computed once when a World's Game and Mods are composed:
  start from what the Biomes lay (ground, soil, ores, surface features), what
  Plants yield and what breaking any of those drops, then add every Recipe
  output whose inputs are already Obtainable, until nothing changes. Structures,
  the Catalogue and Mod Commands are not sources. A breach refuses the World,
  naming the Template, the palette character and the Material. A Grown Material
  or a Door's open Material is refused in a Template; a Door is stored closed.
- Cells carry Material, Shape and Orientation only. Fluids, burning, Container
  contents, entities and loot are out; the format leaves room for them.

Sampling and rotation:

- A World samples a Template by taking, for each of its Voxels, the cell its
  centre falls in, with the same rounding guards and metre arithmetic trees use,
  so the footprint in metres is equal at 1.0, 0.5 and 0.25 m.
- A Structure turns in quarter turns about the vertical axis, picked from the
  hash by worldgen and from the Player's facing by the Tool. Each cell's
  Orientation composes with the turn. No mirroring.

Worldgen:

- A Biome gains a `structures` list, outside the 16 surface-feature slots, of at
  most 4 rows: a Template, a spacing in metres and a chance. Spacing has a floor
  of twice the Template's longer side.
- A new pass runs after terrain, smoothing and ores and before plants. It exists
  only when a Biome lists a Structure; otherwise nothing runs and a World
  generates what it did, with no new Generator version.
- A square lattice of the spacing holds a Structure by a hash of the seed and
  the cell, salted per row so rows of one spacing never share cells. Its
  position within the cell is inset so two never touch, and up to 4 draws are
  tried against the rules. The first that passes wins.
- The rules: every sampled footprint column, a metre apart, is in a Biome that
  lists the Template; the ground varies by at most the tolerance; no lake covers
  a sampled column; no cave cuts a sampled column's surface; the Ground layer
  sits at the footprint's median ground height.
- A non-Keep cell overwrites terrain, ore and lake water, Air included. Layers
  below the Ground layer overwrite underground. The foundation Material fills
  any gap beneath the lowest layer, down to ground.
- The pass reads no Chunk; every Chunk decides every Structure that can reach it
  and writes only its own Voxels, so a Structure is whole from any Chunk's
  viewpoint and a player's edit changes no neighbour's result.
- Trees, scatter and crop fields skip every column under a Structure's footprint
  rectangle. A Structure's acceptance never asks about them.
- Coarse levels leave Structures out and do not vote on them.
- The vertical bounds the Server trusts for streaming and open sky widen by the
  tallest and deepest Template any Biome lists; a World with none keeps its
  exact bounds. Spawn clearance counts Structures.

Settlements:

- A Biome gains a `settlements` list of at most 2 rows, each naming a
  Settlement and a spacing in metres. A Settlement declares a radius of at most
  64 m, an optional centre Template, up to 24 Plots drawn from a weighted list
  of Templates each with a minimum and maximum count, an overall minimum, and a
  path Material and Shape.
- Settlements have their own salted lattice and run before single Structures.
  The claim order is Settlements, single Structures, crop fields, trees and
  scatter, and nothing asks back up the chain. A Settlement's disc keeps single
  Structures out for a further margin, 16 m by default.
- A Settlement is accepted when its centre passes the ground rules. Roads
  branch from the centre along a slope-aware path that avoids water and ground
  too steep to grade. Plots grow beside roads, each Structure turned so its door
  faces the road, by weighted draw within the counts; a Plot whose ground fails
  stays empty, and the Settlement is dropped below its overall minimum.
- Each Plot is cut and filled level under its footprint and a 2 m apron, by at
  most 2 m, blended out to the natural ground. Roads are laid in the path
  Material on the ground, as stairs and slabs where they climb. This edits
  Voxels in the pass; the heightmap is unchanged, so ADR-0022 holds.
- The layout is a pure function of the seed and the Settlement's cell, computed
  once per Settlement per Chunk generation and cheap enough for every Chunk in
  reach to repeat. Single Structures in the wild keep the strict level rule.

Snapshot:

- A World freezes the Templates and Settlements its Biomes reference in the
  generation snapshot,
  by Material and Shape name, in a World-level table the Biome rows index into.
  It is optional on read and absent when empty; an older save loads without it,
  and a Template edited after a World was saved changes nothing in that World.

Tool placement:

- A new Tool kind, `place`, in the closed enum. Its Use answers and applies
  nothing, as ADR-0026 requires: it opens the Template list. Place sends a
  separate Structure intent naming the Template, the anchor from the aim ray and
  the turn. The Client never sends cells; the Server expands them.
- Creative only and free: refused in Survival, spending no Item.
- Only the anchor must be in edit reach. The Server waits up to a bounded number
  of ticks for unloaded or unlit Chunks, as Layout does, then refuses with a
  reason. Placement is all or nothing.
- Cells land through the one World change in per-tick batches with one relight
  per batch, and Mods hear them as Edit events.
- A placement is one undo group. The journal's cap counts cells, 131,072 in all,
  keeping its 128 groups; undo and redo stay all or nothing with their present
  checks. Single-cell Edits stay as undoable as now.
- The aim answer gives a state for each cell so the Client draws the ghost
  without reading World storage. Any UI lands in the mockup and is approved
  before the Client implements it.

Content:

- Each of Terra, Tidewater, Ember, Lumen and Meadow declares fifteen Templates
  from Materials Obtainable in it: ten Settlement buildings and five wilderness
  Structures, as the approved mockup shows, and at least one Settlement that
  uses them; Terra also a town. Stations a building needs (an anvil, a forge, an
  oven) are declared by the Game with the step 16 machinery. The SDK starter
  declares one Template. Existing trees stay parametric.

## Testing Decisions

A good test checks behaviour a Game author or Player can see, through the two
seams, never a private function or a table layout.

Worldgen, mirroring the Plants tests:

- A Structure is whole across Chunk borders, equal in metres at 1.0, 0.5 and
  0.25 m, and the same whichever Chunk is generated first, with more than a few
  that cross a border.
- All four turns compose Orientations correctly, and a Structure's footprint in
  metres is the same turned.
- A Structure never overwrites outside its rules: not in a lake, not on a cave
  mouth, not on ground steeper than its tolerance, only in a listed Biome.
- Trees, scatter and fields skip the footprint, and a Structure ignores them.
- Keep cells leave terrain untouched and Air cells carve.
- A World with no Structure rows generates its recorded digests, bounds and
  spawn clearance, and a version 1 World is unchanged.
- Coarse levels leave Structures out.
- The snapshot round trips, an older save loads without it, and a Template
  edited later changes nothing in a saved World.
- A Biome stays inline within its size limit with 4 rows.
- A benchmark of the 9 surface Chunks, bare against with Structures, at all
  three Voxel sizes, interleaved CPU time, with the margin recorded in
  DESIGN.md.

Settlements:

- A Settlement is whole across Chunk borders, the same whichever Chunk is
  generated first, and equal in metres at all three Voxel sizes.
- Every Plot's door faces a road, no two Plots overlap, and counts stay within
  each Template's minimum and maximum.
- Plots are level within their footprint, the cut and fill never exceeds 2 m,
  and roads never step more than one Voxel's worth of slab per metre.
- No single Structure stands within a Settlement's disc and margin.
- Generating the Chunks of a Settlement costs no more than the agreed margin,
  in the benchmark.

Server and Declaration:

- The Tool is refused in Survival and spends no Item in Creative.
- A placement lands whole or not at all, waits for unloaded Chunks, and refuses
  with a reason after the bound.
- One undo removes the whole Structure and one redo restores it; a Structure
  fills the journal's budget without evicting the wrong groups.
- Mods receive the Edit events.
- A Template using a Material that is not Obtainable, a Grown Material or a
  Door's open Material refuses the World with the Template, cell and Material
  named; adding the Recipe that makes it Obtainable lets the World open.
- A `ground` cell takes grass, sand or snow as the Biome lays.
- A Template with an unknown Material or Shape, an oversize grid, a spacing
  under its floor, a fifth row or an unknown Template in a Biome is a load error
  that names it, from both the data path and a component.

Prior art: the Plants worldgen tests and bench, the crop field tests, the Layout
tests, the surface-feature recorded digests, the coarse generation tests, and
the Mod loader's data tests.

## Out of Scope

- Container loot, entities, fluids and burning in Templates, and anything
  needing Server machinery worldgen cannot run (steps 22 and 23).
- Mirroring a Structure.
- Capturing a Template from the World, and the structure editor (step 25).
- Converting trees to Templates; ADR-0027's parametric trees stay.
- Structures at coarse LOD levels, so a distant landmark is not drawn.
- Placing in Survival or for an Item cost; Blueprints and Sites are step 20b.
- Weathering and variants of a Template (mossy, broken), as rules applied while
  it is placed.
- Unique landmarks fixed at World creation.
- A Mod Command that places a Template.
- Moving or flying bodies built from Templates (step 24).

## Further Notes

- The worktree is `.worktrees/feat+phase-20-structures`, on
  `feat/phase-20-structures`.
- The first ticket is a mockup of the `place` Tool's list, ghost and HUD, and
  the Client work waits for its approval.
- The design was settled by a grilling session; its decisions are in ADR-0028
  and this spec. Where it said Use sends the Structure intent, Place sends it
  and Use only opens the list.
