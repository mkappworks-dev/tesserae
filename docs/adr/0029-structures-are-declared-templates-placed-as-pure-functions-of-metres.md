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
([ADR-0020](0020-data-only-mods-and-development-restarts.md)). Its Template
cells name a Material, a Shape and an Orientation by full `<mod id>:<name>`,
never by id, as the save does. A Template cell's side is in metres, and a World
samples it at its own Voxel size ([ADR-0003](0003-voxel-size-is-per-world.md)).
Authoring in Voxels was rejected: it would pin a Template to one Voxel size, or
refuse every World that differs.

## Worldgen placement is a lattice of pure draws

A Biome lists `structures` rows outside the 16 surface-feature slots. Each
lattice cell of a square lattice `spacing-m` on a side draws, by a hash of the
seed and the lattice cell, a candidate anywhere in it with its rotation and a
priority. It stands only if no neighbouring lattice cell's candidate with a
higher priority lies within the spacing (a pure-function Poisson disc, so
Structures are never on a grid nor clumped), and then only if the ground takes
it; a refused candidate is not tried again elsewhere, since a retry could stand
nearer a neighbour than the spacing. Settlement centres are spread the same way,
kept apart by their discs and margins. The neighbours are asked for candidates
only, never the ground, so the answer is the same from every Chunk. Each Chunk
decides every Structure that can reach it and writes only its own Voxels, so one
is whole across Chunk borders and equal in metres at every size. The pass reads
no Chunk, so a player's edit never changes what a neighbouring Chunk generates.

A Structure's non-`keep` Template cells always overwrite terrain, ore and lake
water. A Biome with no `structures` runs no pass. Trees, scatter and fields skip
a Structure's footprint, and a Structure never asks about them, which avoids a
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

## Amendment, 2026-11-16: as built

Checked against the code at the end of step 20. The decisions above hold; these
are where the build is narrower or more exact than they say.

- **Settlement Plots do not use a Template's level tolerance.** A Plot stands
  where every metre sample of its footprint and a 2 m Apron is dry, no cave cuts
  the footprint's top metre and every sample is within 1.5 m of the lower
  median; it is then cut and filled by at most 2 m. A Settlement's centre
  Template, and every single Structure, keep the strict tolerance rule.
- **A door faces the Road by a rule, not a declaration.** The door is the first
  closed Door Template cell in the lowest layer, from the Ground layer up, that
  holds one, found through the Game's Door voxel-use and frozen in the snapshot.
  A Template with none turns its +Z edge to the Road.
- **Roads are graded, and climb as built-in slabs and stairs.** No steeper than
  0.45 rise over run, so neighbouring columns differ by at most half a Voxel at
  any size; the declared path Shape is used only on level Road.
- **Generation keeps a bounded cache of what each lattice cell stands.** Without
  it every Chunk a Structure or Settlement reaches worked it out again (up to
  0.63 ms a Chunk). Each generator and its clones share one cache, forgotten
  whole when full, whose values are worked out outside its lock from the seed
  and the key alone, so generation stays a pure function of metres in any Chunk
  order and on any thread.
- **Obtainable is over Items, and checks only what Biomes stand.** A Material is
  Obtainable when an Item that places it can be held. Recipe Stations are not
  checked. Only the Templates a World's Biomes list, alone or in a Settlement,
  are checked, since only a Creative Player can place another, and a listed
  Settlement's path Material is checked too. A Grown Material or a Door's open
  Material is refused in the shared validator before Obtainable is worked out.
- **Generation keeps 48 m round the World origin clear.** No single Structure's
  footprint and no Settlement's disc and margin comes within 48 m of x = 0, z =
  0, where Games build their spawn area, such as Ember's lava vents. The check
  is on the candidate, from metres alone, so it is free and leaves a World
  without Structures unchanged.
- **`ground` is a full cube, and never rock.** Its soil runs however deep a
  cellar goes.
- **The snapshot keeps only what a World uses.** It freezes the Templates its
  Biomes and Settlements name, renumbered, not the Game's whole table; the
  `place` Tool reads the Game's current table.
- **The Tool's turn is the Player's choice.** R (shared with Respawn while the
  Tool is held) turns the Structure; the Player's facing does not pick it. The
  Tool lays no foundation. Its footprint's near edge, the side facing the
  Player, sits on the anchor, centred across the Player's view, and the
  Structure extends away from the Player, so it stands in front of them rather
  than around them; an aim straight down centres it on the anchor. The Ground
  layer is level with the anchor.
- **A bridge searches its cell for a gap.** A Template that declares `spans`
  stands only where its span crosses a gap from one dry 2 m landing to the
  other: under the middle third, ground 2 m below the Ground layer, or water
  half a metre deep somewhere. Its lattice cell tries a 2 m grid of points and
  four turns and keeps the deepest gap inside the cell, so its row skips the
  neighbour draw. Under its span its `air` and `ground` cells keep what is
  there, and the foundation fills only under the landings. On the demo Games'
  rolling ground, deep dry valleys at a bridge's scale are rare: the bridges
  found cross water.
- **Chests hold loot, drawn on the Server, not by worldgen.** Worldgen still
  writes Voxels only. A pure query, `Worldgen::stood_at`, says what a worldgen
  Structure stands at a cell and which table applies: a Settlement building's
  own, else the Settlement's, else, in the wild, the Biome's at that column. The
  Server draws from it the first time a Container is needed for a chest whose
  cell still holds that Voxel, and records the cell as looted, in the save, once
  drawn or written over, so a chest put back is a Player's. The draw is keyed by
  the Structure's whole-metre corner and the Template cell, never the Voxel or
  its height, and only a Template cell's first Voxel draws.
- **A Settlement's size is a range each copy draws.** Its most Plots are drawn
  evenly from `plots-min` to `plots` by its lattice cell's hash, and its radius
  follows them from `radius-min-m` to `radius-m`, up to 160 m and 96 Plots.
  Spacing, bounds and claims take the largest radius, so copies never meet. From
  a 66 m disc roads fork every 36 m out, both ways, into streets. A ranged copy
  tries a Template that failed its ground again after the rest, so its size
  follows its draw on rough ground; a fixed-size copy keeps the old order, so a
  World made before ranges lays out the same.
- **The undo journal refuses rather than overflows.** Its cap is 131,072 changed
  Voxels across 128 groups, and a placement over it is refused before landing
  with `TooBigToUndo`, so a placement is always one undo group. An undo lands in
  one tick, and its Edit events past 1,024 in that tick wait in order and reach
  Mods 1,024 a tick after it, so a Mod hears every Voxel of it.
