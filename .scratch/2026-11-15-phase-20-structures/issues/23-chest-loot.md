# 23: Chests in Structures hold loot that fits their place

Status: ready-for-agent

Blocked by: 17, 20

Spec: [Phase 20](../spec.md).

## What to build

A chest that worldgen stands as part of a Structure holds a few random Items,
drawn to suit where it stands. A Settlement's chests hold what that Settlement
would keep; a wilderness Structure's chests hold what that Biome yields.

- **Declaration.** A loot table is a list of
  `{ item, min, max, weight }` entries and a number of rolls (`rolls-min`,
  `rolls-max`).
  - A Settlement may declare `loot`, and so may a Biome.
  - Optionally, a Settlement may give one Template its own `loot`, overriding
    the Settlement's for that building, e.g. the smithy's chest holds tools and
    metal, the bakery's holds bread.
  - The table goes in the WIT (both copies synced), TOML, the validator, the
    snapshot and contract-compatibility "Amended before release".
- **Which table.** For a chest in a Settlement building, the building's own
  table if declared, else the Settlement's. For a chest in a wilderness
  Structure, the table of the Biome at that chest. With no table, the chest is
  empty.
- **Pure and lazy.** Worldgen stays Voxels only. A chest's contents are drawn
  the first time a Container is needed for it: when it is opened, broken, or
  a Mod asks. The draw comes from the World seed, the chest's position and the
  table, so it is the same whenever it happens.
  - The Server asks the generator whether that cell is a worldgen chest: a
    Structure covers it, the Template cell is a chest, and the cell still holds
    it.
  - Once drawn, the Container is saved like any other.
  - A chest a Player places, and every chest of a Structure placed with the
    place Tool, starts empty.
- **Items.** Every Item a table names must be declared, and the loader refuses
  an unknown one. A loot Item does not have to be Obtainable some other way;
  loot is a source.
- **Games.** Each Game declares loot for its Settlement(s) and for each Biome
  that stands wilderness Structures, from its own Items and in its own flavour.
  Examples: Ember's forge hall chest holds ore and ingots; Tidewater's holds
  fish and rope; Terra's town holds more and better than its village.

## Acceptance criteria

- [ ] Opening a worldgen chest in a Settlement building yields Items only from
      that Settlement's or building's table, and in a wilderness Structure
      only from its Biome's, tested through the Server's Use and Container
      path.
- [ ] The same chest yields the same Items whether opened first or broken
      first, at any time, in any Chunk order, and at 1.0, 0.5 and 0.25 m.
- [ ] A Player-placed chest and a place-Tool Structure's chest start empty.
- [ ] Contents persist after the first open, through save and reopen.
- [ ] Every Game declares loot, and each Game's tables load. An unknown Item
      is refused with a clear error.
- [ ] Worlds without Structures are unchanged, and worldgen cost does not
      move.
