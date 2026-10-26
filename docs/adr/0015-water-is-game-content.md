# Water is Game content, and Material id 7 is retired

Amended by [ADR-0016](0016-only-air-is-built-in.md): air is the only built-in
Material, ids 1 to 7 are retired, and the Game names its ground.

Status: accepted. Amends [ADR-0009](0009-materials-are-table-data.md)'s list of
permanent ids.

## Context

The engine owns mechanics and a Game's Declaration owns content: lava, mud and
wood are rows the demo declares, and the engine decides how any fluid flows,
waterlogs, reacts, heats and puts fire out. Water was the exception. It was
Material 7, reserved in step 5 for generated lakes before fluids existed, with a
fluid row built into the engine that extinguishes. So every World had water, the
engine chose its reach and rates, a Game could neither leave it out nor fill its
lakes with something else, and the demo's lava-and-water reaction had to name
`core:water` instead of a Material of its own.

ADR-0009 made ids 0 to 7 permanent because a saved World refers to them by
number. No save exists yet, so dropping one costs little now and a migration
later.

## Decision

**The built-in Materials are ids 0 to 6: air, stone, dirt, grass, lantern, glass
and leaves.** They stay permanent, as ADR-0009 says.

**Id 7 is retired, not reused.** The Material table reports it unknown, the
Registry never lists it, and a Mod's Materials are still numbered from 8 in
registration order. Both keep a slot for it with no Material behind it, so the
ids after it stay where they are: in the table the slot answers as the opaque
stand-in ADR-0009 gives any unknown id, and in the Registry it has no name. A 7
already in a Chunk is therefore drawn as that stand-in, is solid, and is never
read as a fluid. A Command or a player Edit that names it is refused as an
unknown Material. The constant and the row are deleted, not kept as aliases.

**The engine has no fluid row of its own.** The built-in fluid table is empty.
Every fluid, water included, comes from a Declaration and is validated like any
other; nothing in the engine assumes some fluid extinguishes, and the engine's
placeholder palette has no colour for one.

**A World with no Game is dry.** It has no lakes, no fluid and nothing that
burns, so igniting anything is refused as not flammable and the player's default
hotbar holds only the built-in solids.

**`core:water` is an unknown name on every contract line.** It is not mapped to
a Game's water: a behaviour row on 0.3 or 0.2, or a 0.3 lake, that names it
fails the load the way any unknown name does, and the ids every line is handed
at `start` do not include it.

### What this changes in ADR-0012 and ADR-0013

- [ADR-0012](0012-mods-declare-once-and-command-in-batches.md): the 0.3 line's
  standing-water record names its lake fluid because there is no built-in one to
  default to, and a 0.2 or 0.1 Game's standing water, which names none, is
  dropped with a warning. Every 0.1 fixture still passes unchanged, its ids
  included, because a Mod's ids still start at 8.
- [ADR-0013](0013-fluid-and-fire-live-beside-the-voxel.md): a generated lake is
  packed cubes of the Game's lake fluid, read as sources by the same Occupants
  rule, rather than packed cubes of a built-in water. Content identity in its
  durable-state table now includes which fluid fills the lakes, through the
  Game's World parameters.

## Consequences

- A Game declares water as it declares lava, with its own reach, rates,
  waterlogging and extinguishing, and may have water lakes, lava lakes or none.
  The demo declares `demo:water` with the numbers the built-in row had.
- The Material table's dense rows keep one row that is never registered. Its
  cost is five bytes; in exchange no saved id moves and no Mod renumbers.
- `MaterialTable::len` and the Registry's length count the retired slot: they
  are the next id, not the number of Materials.
- Tests and benches that need water register a fixture water through the same
  calls a Declaration takes, so the engine's own suite depends on no built-in
  fluid.
- The `dev` gallery's pool is the Game's lake fluid, and with no lake fluid the
  pool is left out.
- A later save must keep id 7 retired. Tombstones for other ids, and migration
  when a Mod is missing, remain a later decision.
