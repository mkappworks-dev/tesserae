Status: done

# Shared building Materials

Terra builds its Structures from the shared Mods `timber`, `brickwork`,
`glassworks` and a new `workshop`, instead of from Materials of its own, and a
Recipe counts towards Obtainable only when its Station is Obtainable too.

Vocabulary: **Material**, **Item**, **Recipe**, **Station**, **Template**,
**Structure**, **Obtainable** and **Data-only Mod** are in
[GLOSSARY.md](../../GLOSSARY.md). The decision record is a new ADR-0038,
amending
[ADR-0029](../../docs/adr/0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md).
Related: [ADR-0001](../../docs/adr/0001-wasm-mod-boundary.md),
[ADR-0020](../../docs/adr/0020-data-only-mods-and-development-restarts.md) and
[ADR-0033](../../docs/adr/0033-generator-version-3-is-the-only-version.md).

## Problem Statement

Every Game builds its houses from Materials only it declares. Terra declares
fifteen building Materials, four Stations, a chest, doors, glass and a lamp in
its own code, each with its own Recipe, while the shared Mods `timber` (planks,
thatch) and `brickwork` (brick, cobble, roof tile) sit unused: both READMEs say
"Used by: nothing yet" or name only `bog`. Terra's `terra:brick`, `terra:thatch`
and `terra:cobblestone` duplicate `brickwork:brick`, `timber:thatch` and
`brickwork:cobble` outright. A Mod author who wants a building that fits a Terra
village has nothing shared to build it from, and a new Game has to declare the
whole set again.

The Obtainable rule already refuses a World whose Biomes stand a Template with a
Material no Player can get, for every Game. It has two holes:

- **Stations are not checked.** A Recipe at the `oven` counts even when no oven
  is Obtainable, so a Material made only there passes while no Player can ever
  make it.
- **Only Templates a Biome stands are checked.** A Template that only the
  Creative `place` Tool or a later Site can stand is never asked, although the
  glossary says every Template used in a World must be Obtainable.

## Solution

Building Materials come from shared, Data-only Mods. Each declares its
Materials, Items, Recipes and voxel-uses, and makes them from shared world
Materials at shared Stations:

| Mod          | Declares                                                                                        | Made from                                    |
| ------------ | ----------------------------------------------------------------------------------------------- | -------------------------------------------- |
| `workshop`   | the `workbench`, `forge`, `anvil` and `oven` Stations, the `chest` Container, the `table` Shape | `flora:log`, `strata:stone`, `strata:sand`   |
| `timber`     | four planks, two frames, shingles, thatch, awning, the `door` and `gate` Doors, a `leaf` Shape  | `flora:log`, `flora:leaves`, `crops:wheat`   |
| `brickwork`  | cobble, ashlar, slate, brick, tile, red tile, whitewash                                         | `strata:stone`, `strata:clay`, `strata:sand` |
| `glassworks` | glass and the lantern it already has, now with Recipes                                          | `strata:sand`, `flora:log`                   |

Terra depends on all four and its Templates name their Materials. Terra's own
building Materials, Stations, chest, Doors, glass and lamp are deleted. Its
buildings keep their look, because every colour Terra used is kept as a shared
variant and drawn in the shared art; the shared Materials the colour session
already drew (planks, thatch, brick, cobble, tile) keep that art, so those shift
slightly. Terra bridges its own ground to strata with two hand Recipes,
`terra:rock` to `strata:stone` and `terra:sand` to `strata:sand`, since strata
reaches Terra only as bedding.

The Obtainable check counts a Recipe only once a Material carrying the Recipe's
Station tag is Obtainable (a hand Recipe needs none), and it checks every
Template that any loaded Mod declares, not only the ones a Biome stands. A
breach still refuses the World, at Create World and on open, with a message
naming the Mod, the Template and the Material.

The other four demo Games keep their themed building Materials, which are their
identity and already pass the check through their own Recipes. They take the
shared Stations, chest, Doors, glass and lantern. None of their themed Materials
matches a shared one in colour (Meadow's whitewash, fieldstone, clay brick and
straw are the nearest, and each differs visibly), so each Game keeps all of its
own. Retinting shared Materials per Game is a follow-up (see Further Notes).

Existing development Worlds do not migrate: they refuse to open, through the
save's existing refusal of a missing Material and a changed Mod build, as
[ADR-0033](../../docs/adr/0033-generator-version-3-is-the-only-version.md) did
for Generator versions before the first release.

## User Stories

1. As a Survival Player in Terra, I want a village house to be made of Materials
   I can craft, so that I can repair or copy any building I find.
2. As a Survival Player, I want every Station a building Material needs to be
   craftable itself, so that no Recipe in the list is out of my reach forever.
3. As a Survival Player in Terra, I want to make planks, frames, shingles,
   thatch and awnings at a workbench or by hand from logs, leaves and wheat, so
   that wood buildings follow from felling trees and harvesting fields.
4. As a Survival Player in Terra, I want to make cobble, ashlar, slate, brick,
   tile, red tile and whitewash from stone, clay and sand at the oven or the
   anvil, so that masonry follows from mining.
5. As a Survival Player in Terra, I want to turn the rock and sand I mine into
   strata's stone and sand by hand, so that I am not waiting on a rare bedding
   layer before I can build in brick.
6. As a Survival Player, I want to make the workbench, forge, oven and anvil
   from logs, stone and sand, so that the Station chain starts from what I can
   gather by hand.
7. As a Survival Player, I want a lantern and glass windows from sand and logs,
   so that the lit, glazed buildings I find are ones I can build.
8. As a Survival Player in Terra, I want a chest and a door from the shared Mods
   to behave as Terra's did: the chest holds Stacks and the door opens on its
   hinge as one panel, so that nothing I knew changes.
9. As a Player in Terra, I want villages to keep their look after the switch:
   four plank colours, red and orange roof tiles and white walls, so that the
   change reads as the same village.
10. As a Player in Terra, I want breaking and walking on planks, cobble or
    thatch to sound like wood, stone or straw, so that buildings that were
    silent now sound like what they are made of.
11. As a Player opening a chest in a Terra village, I want its loot to hold the
    shared Items, so that what I find builds the houses around me.
12. As a Mod author, I want `timber`, `brickwork`, `glassworks` and `workshop`
    to carry their own Recipes and Stations, so that my Templates can name their
    Materials and stay Obtainable in any Game that loads the Mod.
13. As a Mod author, I want a Recipe of mine to open at the shared `workbench`
    or `oven`, so that I do not declare a Station of my own to add a Recipe.
14. As a Mod author, I want my Game's own themed Station to share a Station tag
    with `workshop`'s, so that either opens the same Recipes.
15. As a Mod author, I want the World refused with a message naming my Mod, the
    Template and the Material when a Template of mine uses something no Player
    can get, so that I find the breach before a Player does.
16. As a Mod author, I want the message to say when a Recipe is out of reach
    only because its Station is, so that I know to add or fix the Station rather
    than the inputs.
17. As a Mod author, I want a Template that only the Creative Tool places to be
    checked too, so that a Site built from it later never asks for an Item no
    Player can hold.
18. As a Game author, I want my Game to name shared building Materials in its
    Templates and sound rows like any other Mod's, so that I reuse them without
    copying them.
19. As a Game author, I want to add a bridge Recipe from my own ground to
    strata's, so that the shared Recipes work in a Game whose ground is its own.
20. As the author of Meadow, I want a later look for the shared Materials of my
    own, so that Meadow can share `brickwork:whitewash` without turning grey;
    until then it keeps its own.
21. As the author of Ember, Lumen, Meadow or Tidewater, I want to keep my themed
    building Materials, so that my Game still looks like itself.
22. As the author of any demo Game, I want my Game to use `workshop`'s Stations,
    chest, Doors and `glassworks`' glass and lantern, so that furnishing is
    shared across Games.
23. As the maintainer, I want CI to compose every Game under `games/` with its
    Mods and run the Obtainable check, so that a breach in Ember, Lumen, Meadow
    or Tidewater fails CI without adding tests for those Games.
24. As the maintainer, I want the mockup to map each building role to its shared
    Material and regenerate every Game's Templates, so that the Templates and
    the mockup cannot drift.
25. As the maintainer, I want `bog`'s reaction product `brickwork:brick` to keep
    working when brickwork gains Recipes and dependencies, so that nothing that
    already used brickwork breaks.
26. As a Player with a development World made before this change, I want it to
    refuse to open with a message naming what is missing, so that it never opens
    with holes where Terra's Materials were.
27. As a reader of the glossary, I want the Obtainable entry to say that a
    Recipe needs an Obtainable Station, so that the rule as written matches the
    rule as checked.
28. As a reader of a Mod's README, I want "Used by" to list the Games and Mods
    that use it, so that I can see the blast radius of changing it.

## Implementation Decisions

### Obtainable

- A Recipe whose Station is not `hand` counts in the fixed-point walk only once
  some Material that a voxel-use declares a Station with that tag is itself
  Obtainable. Station Materials enter the walk like any other: laid, grown,
  dropped or made.
- Every Template any loaded Mod declares is checked, including the ones no Biome
  or Settlement names, and so is every declared Settlement's path Material, used
  or not. The existing exemptions stay: `ground` cells, fluid cells, and a
  crop's later Stage when its first Stage is Obtainable. Grown Materials and a
  Door's open Material stay refused.
- A breach refuses the World as today. When a Material would be Obtainable but
  for a Station, the message names the Station tag.
- This amends ADR-0029's "Recipe Stations are not checked" and "only the
  Templates a World's Biomes list" in a new ADR-0038, which also records the
  shared-Mod decision below.

### The shared Mods

All four are Data-only Mods. They add no host call and nothing per Voxel, so
architectural invariant 2 holds, and the only Engine change is the Obtainable
rule on the Server.

| Mod          | `depends`                    |
| ------------ | ---------------------------- |
| `workshop`   | `flora`, `strata`            |
| `timber`     | `flora`, `crops`, `workshop` |
| `brickwork`  | `strata`, `workshop`         |
| `glassworks` | `strata`, `flora`, `workshop` |

`bog` already depends on `brickwork` and now pulls in `strata` and `workshop`
through it.

Recipes. Each row reads inputs, then output count, then Station:

| Output                                        | Inputs                      | Count | Station   |
| --------------------------------------------- | --------------------------- | ----- | --------- |
| `workshop:workbench`                          | `flora:log` ×4              | 1     | hand      |
| `workshop:forge`                              | `strata:stone` ×8           | 1     | hand      |
| `workshop:oven`                               | stone ×4, `strata:sand` ×2  | 1     | hand      |
| `workshop:anvil`                              | stone ×6                    | 1     | forge     |
| `workshop:chest`                              | log ×8                      | 1     | workbench |
| `timber:planks`, `pine_planks`, `dark_planks` | log                         | 4     | hand      |
| `timber:red_planks`                           | log                         | 4     | workbench |
| `timber:frame`                                | log                         | 1     | hand      |
| `timber:dark_frame`                           | log                         | 1     | workbench |
| `timber:shingles`                             | `timber:planks` ×2          | 4     | workbench |
| `timber:thatch`                               | `crops:wheat` ×2            | 2     | hand      |
| `timber:awning`                               | wheat ×2, `flora:leaves` ×2 | 2     | workbench |
| `timber:door`                                 | log ×3                      | 1     | workbench |
| `timber:gate`                                 | log ×2                      | 1     | workbench |
| `brickwork:cobble`                            | stone                       | 1     | hand      |
| `brickwork:ashlar`                            | stone ×2                    | 2     | anvil     |
| `brickwork:slate`                             | stone ×2                    | 3     | anvil     |
| `brickwork:brick`                             | `strata:clay` ×2            | 2     | oven      |
| `brickwork:tile`                              | clay, sand                  | 4     | oven      |
| `brickwork:red_tile`                          | clay ×2                     | 4     | oven      |
| `brickwork:whitewash`                         | sand, stone                 | 2     | oven      |
| `glassworks:glass`                            | sand ×2                     | 1     | oven      |
| `glassworks:lantern`                          | `glassworks:glass`, log     | 2     | workbench |

Every input is a shared Item, so no shared Recipe depends on a Game. The anvil
is stone, not iron or basalt: no shared Mod declares iron, and Lumen lays no
basalt, so a basalt anvil would refuse it. The oven is stone and sand, not
brick, so `workshop` does not depend on `brickwork`.

Shapes. `workshop` declares `table`, a thick top on four legs that a body
collides with whole, and the workbench fixes it. `timber` declares `leaf`, a
panel 3/16 thick against its own +Z, which its Doors fix so they hinge and fold
as Terra's did, and which Templates' shutters and boards take; its existing
centred `panel` and `beam` stay, offered to the Player, as a Shape a Door fixes
is not. The door is 2 m
wide and 3 m tall and the gate 1 m by 1 m, as Terra's are, so Templates' door
openings still fit and a Player's body passes with room.

Existing names win a clash: `brickwork:brick`, `brickwork:cobble`,
`brickwork:tile`, `timber:planks` and `timber:thatch` are kept, and the colour
session's art for them stands.

### Terra

Each Terra Material maps to one shared Material:

| Terra                                          | Shared                     |
| ---------------------------------------------- | -------------------------- |
| `oak_planks`                                   | `timber:planks`            |
| `spruce_planks`                                | `timber:pine_planks`       |
| `dark_oak_planks`                              | `timber:dark_planks`       |
| `red_planks`                                   | `timber:red_planks`        |
| `oak_log`                                      | `timber:frame`             |
| `dark_log`                                     | `timber:dark_frame`        |
| `shingles`, `thatch`, `awning`                 | `timber:` the same name    |
| `door`, `gate` and their open states           | `timber:` the same name    |
| `cobblestone`                                  | `brickwork:cobble`         |
| `ashlar`, `slate`, `brick`, `whitewash`        | `brickwork:` the same name |
| `clay_tiles` (red)                             | `brickwork:red_tile`       |
| `terracotta_tiles` (orange)                    | `brickwork:tile`           |
| `glass`                                        | `glassworks:glass`         |
| `lamp`                                         | `glassworks:lantern`       |
| `workbench`, `forge`, `anvil`, `oven`, `chest` | `workshop:` the same name  |

- Terra's `depends` gains `timber`, `brickwork`, `glassworks` and `workshop`.
- Terra deletes every building Material in the table, their Recipes and
  voxel-uses, and its `panel` and `table` Shapes. It keeps `fence`, which its
  paddocks and gardens use with `timber:planks`.
- `terra:glass` and `terra:lamp` are deleted. `terra:ash` stays, since the
  `ash_depths` cave Biome lays it and burning leaves it, and so does
  `terra:iron`, an ore.
- Terra's mud and lava reaction makes `brickwork:brick`.
- Terra adds two hand Recipes, `terra:rock` to `strata:stone` and `terra:sand`
  to `strata:sand`, one for one.
- Terra's loot tables name the shared Items. Its Settlement path is
  `brickwork:cobble` where it was `terra:cobblestone`.
- Terra's material sound rows name the shared Materials. Only brick, glass and
  the lamp had a sound set before; the rest were silent, and now take one as a
  deliberate addition: planks, frames, shingles, Doors, chest and workbench as
  wood; masonry, the forge and the oven as stone; the anvil, glass and the
  lantern as metal; thatch and awning as soft.
- The mockup keeps its role table keyed by Terra's role names and maps each to
  its shared Material, failing on a role with none; the other Games' per-Game
  substitutions are unchanged. The furnishing table and the exporter's Shape
  prefixing name the shared furnishing, `timber:leaf` and `workshop:table`.
  Every Game's Templates are regenerated. Templates are not UI, so no separate
  mockup approval.
- The guest build's list of Mods shipped with Terra, and the mockup's exported
  Material gallery, follow Terra's new `depends`.
- Terra keeps building against the WIT contract alone, and its boundary check
  passes.

### The other demo Games

- Ember, Lumen, Meadow and Tidewater depend on `workshop`, `timber` and
  `glassworks`, and drop their own `workbench`, `forge`, `anvil`, `oven`,
  `chest`, `door`, `glass` and `lamp` for the shared ones. Their Templates name
  the shared ones.
- Every other themed building Material stays the Game's own, with its own
  Recipe, now at the shared Stations.
- The shared demo building kit stops generating Stations, chests, Doors, glass
  and lamps, and its `panel` and `table` Shapes.

### Docs

- ADR-0038 as above, linked from ADR-0029 as "Amended by ADR-0038".
- The glossary's Obtainable entry gains: a Recipe counts only when its Station
  is Obtainable.
- The authoring guide's Obtainable steps and its Door example (which names
  Terra's `panel` and `table`) are updated.
- Each shared Mod's README lists its Recipes and "Used by", written by the
  ticket that changes the Mod; the Mod table in the Mods README and the demo
  Games doc are updated at the end.

## Testing Decisions

A good test composes a World's Mods through the Server's load, as the existing
Obtainable tests do, and asserts on whether the World is refused and on the
message. It never reaches inside the fixed-point walk.

- **Obtainable rule.** Beside the existing Obtainable tests in the Server's mod
  loading: a Recipe at a Station nobody can get makes its output not Obtainable,
  and the message names the Station tag; a Station made by a hand Recipe opens
  its Recipes; a hand Recipe needs no Station; a Template no Biome names is
  checked and refused; the existing exemptions still hold.
- **Shared Mods.** CI's existing `validate` pass composes each with its
  `depends`. That proves the names resolve but lays nothing, since only a Game's
  ground and Biomes lay Materials, so Obtainable is tested by composing the real
  shared Mods under a small data-only fixture Game whose ground lays stone, sand
  and clay and whose Biome stands a tree and a wheat field, and asserting every
  shared Material is Obtainable through its Stations. The shared-Mods inventory
  test lists `workshop`, with its thumbnail.
- **Terra.** Terra is composed through load and passes; the existing Terra
  building test, which walks Terra's building and Station Materials, is
  rewritten over the shared Materials Terra uses and keeps asserting each is
  Obtainable through a Station that is. The Templates-match-the-mockup test, the
  Terra `depends` test and the loot test are updated. A test names a Template
  Material from each shared Mod and asserts it resolves.
- **Other demo Games.** No new tests (demo-Game tests run on Terra only). CI's
  validate step runs over `games/*/` as well as `mods/*/`, after building the
  guests for the WASM target and trusting them for the run, so a breach in any
  Game fails CI.
- Prior art: the Obtainable tests beside the Server's mod loading, the demo
  Games' building and structures tests, and the author command's validate tests.

## Out of Scope

- Retinting shared Materials per Game. `[[appearance]]` takes only a Mod's own
  Material today; a Game-supplied look for another Mod's Material is the
  follow-up that would let Ember's planks be `timber:planks` drawn sooty.
- Switching Terra's ground (`terra:stone`, `dirt`, `sand`) to strata's. It
  changes the terrain's look and belongs with the colour work, not here.
- Recipe inputs by tag (`#stone`). The bridge Recipes cover Terra; revisit when
  a third Game needs bridges.
- A shared iron or ore Mod.
- Redrawing the art the colour session (`fix/colour-art-and-shader`) draws for
  `timber`, `brickwork`, `flora`, `strata` and `crops`.
- Migrating saved Worlds.
- Blueprints and Sites (step 23); this only makes sure their Templates will be
  Obtainable.

## Further Notes

- Checking every Template found the `lily_pond` outskirts plot in Terra and
  Tidewater, which the old check skipped: nothing laid or made its lily pads,
  so `aquatic` gained a hand Recipe, two seagrass to one lily pad.

- The art for the new shared Materials is drawn in the Mod art generator after
  the colour branch merges, in that session's style, with Terra's current look
  as the target so its villages read the same. Nothing already drawn is redrawn.
  The art ticket waits on that merge; the content tickets do not, and ship with
  placeholder textures until it lands.
- Station tags are global strings; two Mods' Stations sharing a tag is how a
  Game's themed bench and `workshop`'s open the same Recipes.
- Any rebuild of Terra already refuses its old Worlds through the build hash, so
  the switch changes nothing about migration in practice.
