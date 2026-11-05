Status: done

# Phase 13: Biomes, richer worldgen

DESIGN.md roadmap step 13. The biome and climate seam, with the generator
version pinned in the save. A Game or Mod supplies Biomes, ores and Biome caves
as data. Step 13 feeds the sky and fog colours from step 9.

Vocabulary: **Climate**, **Biome** and **Generator version** are defined in
[GLOSSARY.md](../../GLOSSARY.md).

## Problem Statement

Every World generates the same three-layer ground, one cave field and one haze
colour, wherever you stand. A Game author cannot make a desert next to a tundra,
cannot place ore, and cannot tint the sky and fog to match the land. Later steps
(weather, plants, the map, structures) have nothing to ask "what kind of ground
is this?". And nothing stops a future generator change from silently reshaping
the terrain of a World that already exists, for the same seed.

## Solution

A World's ground is chosen column by column from a Biome, which is chosen from
Climate: temperature and humidity fields that are a pure function of the seed
and a position in metres. A Game or Mod declares its Biomes, its ores and the
Climate scale as data. A Biome owns ground Materials, ores, cave tuning, and a
sky and fog colour pair. As the player walks between Biomes the Server tells the
Client the colour pair for where the Eye is, and the sky and fog ease to it
together.

A World pins its Generator version and a snapshot of its Biome table in the
save, so adding a Biome to a Game later, or revising the generator, never
changes an existing World's terrain for the same seed.

## User Stories

1. As a Game author, I want to declare Biomes as data, so that my Game has more
   than one kind of ground without writing generator code.
2. As a Game author, I want to declare the Climate scale in metres, so that
   Biome regions are the size my Game needs at any Voxel size.
3. As a Game author, I want a Biome to name its ground Materials, so that a
   desert is sand and a tundra is snow.
4. As a Game author, I want a Biome that omits its ground to fall back to the
   Game's `world.ground`, so that I only declare what differs.
5. As a Game author, I want to declare an ore with a Material, a depth band in
   metres, a vein size and a rarity, so that rock is not uniform.
6. As a Game author, I want to restrict an ore to named Biomes, so that
   different regions hold different resources.
7. As a Game author, I want a Biome to scale the cave field, so that one region
   is riddled with caves and another is solid.
8. As a Game author, I want a Biome to switch caves off, so that a flat plain
   stays walkable.
9. As a Game author, I want a Biome to carry a sky colour and a fog colour, so
   that the air matches the land.
10. As a Mod author, I want to add Biomes and ores to a Game's World, so that a
    data Mod can enrich any compatible Game.
11. As a Game author with no Biomes declared, I want today's behaviour exactly,
    so that existing Games and data-only Mods keep working.
12. As a player, I want different regions to look different, so that exploring
    is worth it.
13. As a player, I want sky and fog to change as I cross into another Biome, so
    that I notice where I am.
14. As a player, I want sky and fog to ease, not snap, so that a border is not
    jarring.
15. As a player, I want the fog and the sky behind it to stay matched, so that
    the terrain never ends in a visible edge against the sky.
16. As a player, I want underwater fog to still win under the surface, so that
    Biome colours do not break the water view.
17. As a player, I want Biome borders to be irregular, so that regions look
    natural, not drawn with a ruler.
18. As a player, I want ore to appear only in rock, so that it never floats in
    air or sits in water.
19. As a player, I want distant terrain at any LOD level to match the nearby
    Biome ground, so that regions do not change colour as I approach.
20. As a player with an existing World, I want its terrain unchanged after an
    update, so that my builds are not buried or floating.
21. As a player with an existing World, I want it to keep opening after the
    Generator version moves, so that an update does not strand my save.
22. As a player with an existing World, I want a Biome added to my Game later to
    leave that World's terrain alone, so that a Game update is safe.
23. As a player creating a new World, I want it to use the current generator and
    the Game's Biomes, so that new Worlds get the new content.
24. As a player, I want a given seed to always produce the same Biomes and ore at
    the same place, so that a seed can be shared.
25. As a player at any Voxel size, I want the same Biome layout in metres, so
    that the World does not change shape with Voxel size.
26. As a Server developer, I want a pure `biome_at(x_m, z_m)` on the generator,
    so that weather, plants, the map and structures can ask what Biome a column
    is.
27. As a Server developer, I want Biome selection to be cheap and `Send + Sync`,
    so that off-thread streaming and per-LOD-level generators stay fast.
28. As a Server developer, I want the surface height band unchanged by Biomes,
    so that vertical streaming extent and open-sky relight stay sound.
29. As an Engine maintainer, I want the Generator version separate from entity
    storage, so that step 12 and step 13 merge without touching each other's
    save fields.
30. As an Engine maintainer, I want the Generator version stamp to be read by the
    generator, so that a pinned World actually uses its pinned algorithm.
31. As an Engine maintainer, I want a regression test of version 1 output, so
    that the pin is proven, not assumed.
32. As an Engine maintainer, I want the Client to hold no generator, so that
    the Server stays the sole authority over World facts.
33. As an Engine maintainer, I want no Mod to reach the generator directly, so
    that Biomes cross the WIT contract as one declaration.
34. As a reviewer, I want each Game's palette in this spec, so that I approve
    colours before they are implemented.
35. As a contributor, I want the glossary, an ADR and the contract compatibility
    doc updated, so that the next reader finds the decision.

## Implementation Decisions

Settled in a grilling session (Q1-Q18).

**Selection and Climate**

- Climate is two continuous fields, temperature and humidity, from the existing
  value-noise fbm with their own seed offsets, sized by Game-declared feature
  sizes in metres. No altitude term in v1; altitude can become a third axis
  later without changing the declaration shape.
- A Biome is a point in the 0-1 temperature/humidity square. The nearest point
  wins. The border between Biomes is warped by low-frequency noise so it is not
  straight.
- Everything is a pure function of the seed and (x_m, z_m) in metres, so it
  holds at every Voxel size and at every LOD level (ADR-0003, ADR-0011).
- `biome_at(x_m, z_m)`, and the Climate values, are a Server-only query on the
  generator in `tesserae-core`. They are not in WIT. Weather (17), plants (18),
  the map (19) and structures (20) build on them.

**What a Biome owns**

- Ground Materials (rock, soil, surface), ores, cave tuning, and a sky and fog
  colour pair. It never changes surface height, relief or sea level in v1, so
  the surface bounds, the vertical streaming band and the half-voxel sea-level
  shift for coarse LOD stay valid.
- Ground Materials and ores switch hard at a border. Cave tuning and the colour
  pair blend smoothly by distance to neighbouring Biomes, so there are no steps
  in fog or cave density.
- A Biome that omits `ground` uses the Game's `world.ground`.
- With no Biomes declared the World behaves exactly as today: `world.ground`,
  the global caves, the haze colours. The Engine ships no built-in Biome
  (ADR-0016).

**Ores**

- An ore is a data record: Material, depth band below the surface in metres,
  vein size in metres, rarity, and the Biomes it appears in.
- A vein is a pure 3D noise blob in metres. It replaces only the rock layer:
  never soil, surface, water or air.
- LOD levels 3 and up, which are generated column-only, carry no ores, as they
  carry no caves. Levels 1 and 2 are derived from level 0 as now.

**Biome caves**

- A Biome multiplies the existing cave field parameters (density, size) or
  switches caves off. There is one carver, so one cost model and one set of LOD
  rules. Caves only turn solid into air, so the height band stays valid.

**Save and the pin**

- Two layers. The Biome table is snapshotted in the save's generation
  parameters as an optional field, absent in version 1. The Generator version
  moves to 2, the save accepts versions 1 and 2, and the saved version is passed
  into the generator, which keeps the version 1 path intact.
- Version 1 reproduces today's output bit for bit and has no Biomes. Version 2
  with an empty Biome table equals version 1. New Worlds are stamped 2.
- A Biome added to a Game later cannot change an existing World, because the
  World's table is its own snapshot. A generator revision cannot change it,
  because the World keeps its Generator version.
- The version field and the Biome snapshot are additive and separate from
  entity storage (step 12).
- We store parameters and a version number, never an engine-internal layout
  (the Minecraft MC-240834 lesson).

**Declaration and contract**

- Edit contract line 0.3 in place, since it is unreleased: append one field at
  the end of the declaration record for Biomes, ores and Climate. Add `[[biomes]]`,
  `[[ores]]` and `[world.climate]` to the Data Declaration. Add a one-line
  "added in place" note to the contract compatibility doc. Step 12 appends its
  own item definitions the same way; whichever merges second rebases that one
  record.
- Only a Game declares Climate, as it declares `world` today. A Mod may add
  Biomes and ores to a Game's World.

**Sky and fog**

- The Server computes the Biome blend at each Eye's position and sends a sky and
  fog colour pair when it changes. The Client eases towards it through the
  existing fog settings resource. The pair always moves together. Underwater fog
  stays an override on top.
- The Client holds no generator and reads no World storage.

**Content**

- All five Games (ember, lumen, meadow, terra, tidewater) declare three Biomes
  of their own flavour, using Materials from the shared Mods they already
  depend on. A Game adds a dependency (for example `frost` or `crystals`) only
  where a Biome needs it. Terra stays dependency-free and declares its own sand,
  snow and ore. No new shared Mods.

**Proposed palette, for approval before the sky/fog and content tickets**

Baseline today is sky 126,146,168 and fog 136,160,190. Fog is always a little
lighter and bluer than sky, as in the baseline. Biome ground Materials are
finalised in the content ticket from each Game's declared dependencies.

| Game      | Biome         | Sky (sRGB)    | Fog (sRGB)    |
| --------- | ------------- | ------------- | ------------- |
| terra     | Temperate     | 126, 146, 168 | 136, 160, 190 |
| terra     | Dry           | 176, 160, 128 | 196, 178, 142 |
| terra     | Cold          | 150, 164, 184 | 176, 190, 208 |
| ember     | Ash flats     | 112, 106, 106 | 130, 122, 120 |
| ember     | Scorched      | 150, 102, 78  | 176, 120, 90  |
| ember     | Highland      | 120, 134, 152 | 138, 152, 172 |
| lumen     | Garden        | 130, 150, 160 | 146, 168, 180 |
| lumen     | Arid court    | 172, 158, 132 | 192, 176, 146 |
| lumen     | Frost court   | 152, 166, 188 | 178, 192, 212 |
| meadow    | Meadow        | 128, 152, 172 | 142, 168, 192 |
| meadow    | Heath         | 150, 148, 150 | 170, 168, 170 |
| meadow    | Dunes         | 174, 160, 134 | 194, 178, 150 |
| tidewater | Lagoon        | 118, 150, 170 | 132, 172, 192 |
| tidewater | Salt flats    | 170, 164, 150 | 190, 184, 166 |
| tidewater | Cold shallows | 144, 160, 180 | 168, 186, 206 |

**Tickets (vertical slices, in order)**

1. Pin the Generator version, with the regression tests written first.
2. Climate and Biome selection in `tesserae-core`, including `biome_at`.
3. Declaration, WIT and the data parser for Biomes, Climate and ores.
4. Ground per Biome and the hard warped border.
5. Ores.
6. Biome cave tuning.
7. Server-sent sky and fog.
8. Content for the five Games.
9. Docs: glossary, ADR, contract compatibility and the DESIGN.md status.

ADR-0022, "Biomes are column content chosen by Climate, and a World pins its
generator version", takes the next free number at merge if step 12 claims that
one first. It records the hard-to-reverse choices: Biomes cannot shape height,
and the save snapshots the Biome table.

## Testing Decisions

A good test checks external behaviour: the Chunk a generator returns, or the
terrain a reopened save reproduces. It does not assert on noise internals,
private helpers or how Biomes are stored in memory.

Two seams:

1. **The generator's chunk function in `tesserae-core`.** Prior art:
   `tesserae-core/tests/coarse_generation.rs` and the surface-equality-across-
   Voxel-sizes tests in the generator's own test module. Covers: determinism for
   a seed; version 1 byte-identical to today (a fixed seed, params and Chunk set,
   written before the biome code); version 2 with an empty table equals version
   1; Biome selection at known Climate points; the same layout at different Voxel
   sizes; ores only in rock and only inside their band and Biomes; cave tuning
   and "off"; the warped border; coarse LOD levels matching nearby ground.
2. **The save round trip through opening a World.** Prior art: the persistence
   and `Commit` tests in `tesserae-core` save module and the Server open-world
   tests. Covers: a version 1 save reopens and reproduces its terrain after the
   Game gains a Biome; a version 2 save carries its Biome table; an unknown
   version is refused.

Declaration parsing and the sky and fog message get small unit tests beside
their code, not their own seam. Review-only screenshots per Biome (offscreen, in
one `/tmp` folder, never committed, deleted after review) check the palette.

## Out of Scope

- Biomes that change surface height, relief or sea level.
- An altitude or lapse-rate Climate term, and more than two Climate axes.
- Migrating version 1 saves to version 2 terrain; a version 1 World keeps its
  version 1 terrain.
- Weather, plants, the map and structures, which use `biome_at` later.
- A HUD biome readout, a debug overlay, or any menu or World-creation UI change.
- Biomes in WASM Mods calling the generator, or any per-voxel host call.
- New shared Mods; per-biome water level; Biome-specific fluids.
- Dropped items and entity storage (step 12).

## Further Notes

- Step 12 has since merged (#55), and the branch is rebased on it. It followed
  the same pattern this spec uses: an optional `entities` field that changes
  neither the format nor the generator version, `items` appended to the 0.3
  declaration, and an "Amended before release" section in the contract
  compatibility doc. Biomes append beside `items` and extend that section.
- Step 12 ran in parallel on `feat/phase-12-dropped-items`. Shared surfaces:
  the save format (keep the version field additive, separate from entities),
  the 0.3 declaration and its compatibility doc (touch only what Biomes need),
  and server tick and plugin wiring (keep worldgen changes inside worldgen
  modules).
- The generator today has no ores, no Biomes, and receives no saved version.
  The save already holds a Generator version, refused unless it matches exactly.
- Research summary: Minecraft (multi-noise, datapack Biomes, new chunks only,
  a serialised layout in `level.dat` that broke an upgrade), Luanti (heat and
  humidity points, mapgen pinned per World), Vintage Story (Climate fixed at
  creation, changes affect new chunks only), Hytale (old generator kept beside
  the new one). No engine's sky and fog handling was verifiable.

## Comments

> _This was generated by AI during triage._

The maintainer approved the palette table as written at triage. Tickets 01-09 are `ready-for-agent`.
