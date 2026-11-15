# 16: Building Materials, textures and Stations per Game

Status: ready-for-agent

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
- [ ] Each Material has a Recipe or a worldgen source, and the Catalogue lists
      it in Creative.
- [ ] The anvil, forge and oven are Stations with Recipes, usable as the
      workbench is.
- [ ] The Block gallery shows every new Material from its exported textures.
