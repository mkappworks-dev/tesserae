# 23: Chests in Structures hold loot that fits their place

Status: done

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

- [x] Opening a worldgen chest in a Settlement building yields Items only from
      that Settlement's or building's table, and in a wilderness Structure
      only from its Biome's, tested through the Server's Use and Container
      path.
- [x] The same chest yields the same Items whether opened first or broken
      first, at any time, in any Chunk order, and at 1.0, 0.5 and 0.25 m.
- [x] A Player-placed chest and a place-Tool Structure's chest start empty.
- [x] Contents persist after the first open, through save and reopen.
- [x] Every Game declares loot, and each Game's tables load. An unknown Item
      is refused with a clear error.
- [x] Worlds without Structures are unchanged, and worldgen cost does not
      move.

## Comments

Where it lives: `tesserae-core/src/loot.rs` (`LootTable`, `LootEntry`,
`validate`, `draw`); `worldgen/loot.rs` (`Worldgen::with_loot`, `stood_at`,
`Stood`, `Loot`); `Settlement::loot` and `PlotRule::loot`; snapshot
`SavedSettlement::loot`, `SavedPlotRule::loot`, `SavedBiome::loot`,
`Generation::with_biome_loot` / `biome_loot`, `Commit::looted`. Loader:
`tesserae-mods/src/loot.rs` (`biome_loot`, `ModError::BadLoot`),
`DeclaredSettlements::check_loot`, both run at the end of `declare_all`. Server:
`Containers::draw_loot`, `LootSource`, `ChunkStore::note_container_left` (noted
in `Batch::set_occupants`, drained by `container::watch`). Tests:
`server/container/tests/loot.rs`, `tesserae-mods` `tests/settlements.rs`,
`tests/settlement_snapshot.rs`, `demo_games/structures.rs`.

Decisions:

- Biome loot is not on the core `Biome`, which must stay `Copy`; it rides
  beside it as `Vec<Option<LootTable>>` by Biome id, in `LoadedMods`,
  `WorldParameters.3` and the generator, and on `SavedBiome` in the snapshot.
- Only `settlement-plot` rows carry their own `loot`; a Settlement's centre
  uses the Settlement's table. No centre Template holds a chest yet.
- The draw is keyed by the Structure's whole-metre corner (x, z) and the
  unturned Template cell, not the Voxel or its height: a Structure's height
  differs by part of a metre between Voxel sizes. Only a Template cell's first
  Voxel, lowest on every axis, draws; the rest of a 0.5 m or 0.25 m chest cell
  open empty. Template indices are left out of the key, as a reopened World
  renumbers them.
- "Edited" is recorded per cell as `looted` in `Containers` and the commit:
  set when a worldgen chest draws, or when any write takes a Container Voxel
  off a cell (the one World write notes it, capped at 4,096 a tick like the
  journal). Chunk revisions are per Chunk, too coarse for this.
- Breaking an unopened worldgen chest is allowed and spills its draw, as the
  ticket's "broken" asks; an opened chest that holds Stacks is still refused
  as before. Fire and Mod Commands spill alike.
- A Container held from a save that predates loot keeps what it holds and
  draws nothing.
- Draws filter Items the World no longer has and clamp to the stack; the
  loader already refuses an unknown Item or a `max` over the stack.
- No Mod reads a Container yet, so "a Mod asks" has no path to draw from.
- No fish, rope, ore or ingot Items exist: Ember's forge hall holds iron
  sheet, basalt and obsidian; Tidewater's net loft sailcloth and kelp. Terra's
  town rolls 3 to 6 against the village's 1 to 3, with iron, glass, lamps and
  pies. Own tables: Terra's blacksmith and bakery; Ember's forge hall, smelter
  and charcoal kiln; Tidewater's net loft, boatwright and smokehouse; Lumen's
  crystal vault, glassworks and lens kiln; Meadow's bakery, tinker and
  granary. Every Biome of every Game has a table.
- Bench (`structures`, interleaved CPU time, A/B against cf929de0): every
  variant's ratio to bare is the same or lower at 1, 0.5 and 0.25 m.

Known gap: a place-Tool Structure landing exactly the same chest Voxel on an
untouched worldgen chest leaves it unchanged, so that chest still draws.

For ticket 22: tests find chests through each Game's `<game>:chest` Material
and whole Templates, not fixed cells, so moved chests need no test change.
