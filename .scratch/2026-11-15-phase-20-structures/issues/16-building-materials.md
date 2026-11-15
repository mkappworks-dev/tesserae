# 16: Building Materials, textures and Stations per Game

Status: ready-for-human

Blocked by: 12

Spec: [Phase 20](../spec.md).

## What to build

Each Game declares the building Materials its Templates use, about sixteen
each, with their own textures in the Game's art style: planks, logs, plaster,
cobble, dressed stone, roof tiles, thatch, slate, shingles and awning cloth, as
the approved mockup paints them (Terra: oak, spruce, dark oak and red planks,
oak and dark logs, whitewash, cobblestone, ashlar, clay and terracotta tiles,
thatch, slate, shingles, awning; the other Games their own set, such as
Tidewater's driftwood, palm log, sandstone, coral brick, shell tiles and
sailcloth). Each new Material is Obtainable: a Recipe makes it from what the
Game's worldgen gives, or worldgen lays it. The Stations the buildings hold (an
anvil, a forge, an oven) are declared with the step 16 machinery and their own
textures and Recipes. The mockup's painted textures are stand-ins; the Games
ship PNG art.

## Acceptance criteria

- [ ] Every Material the mockup's Templates name exists in its Game with a
      texture and passes the Obtainable check.
- [x] Each Material has a Recipe or a worldgen source, and the Catalogue lists
      it in Creative.
- [x] The anvil, forge and oven are Stations with Recipes, usable as the
      workbench is.
- [x] The Block gallery shows every new Material from its exported textures.

## Comments

- Built on 2026-11-15. Every building Material the mockup's Templates name
  exists in its Game with a texture: 15 in Terra, 16 in each other Game, plus
  an anvil, forge and oven Station in every Game. Names use `_` where the
  mockup used `-` (`terra:oak_planks`): Material names allow only `a-z0-9_`.
  The mockup's `gameMaterial` now maps to the real names.
- Criterion 1 is not ticked, for two reasons:
  - Its "passes the Obtainable check" half awaits ticket 12. Until then
    `demo_games/building.rs` stands in for it: it computes Obtainable the way
    the spec says (what Biomes lay, ores, surface features and their Plants'
    yields, then Recipes to a fixed point) and finds every new Material and
    Station Obtainable in every Game. Ticket 12's real check should agree;
    ticket 17 then proves it through the Templates.
  - The mockup's Templates in Tidewater, Ember, Lumen and Meadow also name
    Terra's furnishings: `terra:door`, `terra:chest`, `terra:workbench`,
    `terra:glass` and `terra:lamp` (FIXED in `mockup/structures.js`). Those
    Games do not load Terra, so these do not exist there. This ticket's list
    is building Materials and Stations, so they were not added. Needs a
    decision: declare a door, chest, workbench, glass and lamp per Game
    (here, or in ticket 17), or have ticket 17 use what each Game has
    (`glassworks:glass` and `glassworks:lantern` in Lumen and Meadow). Hence
    `ready-for-human`; everything else is done.
- Recipes: woods, logs, cobble, thatch and cloth by hand from `flora:log`,
  `flora:leaves`, the Game's rock and `crops:wheat` (Meadow); dressed stone
  and slate at the anvil; plaster, tiles and brick at the oven; Ember's iron
  sheet at the forge. The forge and oven are made by hand (at Terra's
  workbench in Terra) and the anvil at the forge, from each Game's own ore.
  Terra's furnishings are unchanged.
- Textures are painted in Rust from the mockup's patterns and colours: the
  four demo Games share `demo_kit::building` (and its `building!` macro);
  Terra keeps its own copy in `games/terra/src/building.rs`, because
  `scripts/check-terra-boundary` allows it the WIT contract alone. The forge
  emits light 6.
- `export-blocks` now exports all five Games to `mockup/blocks/<game>/`, with
  one `index.json` listing `games` and each Material's `game`. Terra's files
  moved from `mockup/blocks/` to `mockup/blocks/terra/`. The gallery groups
  Materials by Game; `structure-materials.js` dropped its painted stand-ins
  and draws the exported textures, and the smithy's anvil cell is now the
  Game's own anvil instead of `terra:iron`.
- `the_demo_games_textures_are_unchanged` was re-pinned. Before re-pinning,
  the fingerprint of every pre-existing texture was checked equal to its old
  pin in all five Games, so no existing art moved.
