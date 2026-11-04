# Phase 12: dropped items, pickup

Status: done

Roadmap step 12 in `DESIGN.md`: the entity system, with Server simulation,
pickup and save. The Game or a Mod supplies item definitions.

## Problem Statement

Removing a Voxel makes it vanish. Nothing in a World exists except Voxels, the
Player and fluid, so there is nothing to collect, nothing to carry, and no
mechanism for the later steps (inventory, crafting, mobs) to stand on. A Game
author cannot say what an item is.

## Solution

A Player who removes a Voxel whose Material a Game or Mod ties to an item sees
that item pop out, fall, settle on the ground and spin gently. Walking close
picks it up: it leaves the World and the Player carries it, counted by name. If
the Player quits and returns, the items still on the ground are where they were
and the carried count is unchanged. Everything is decided by the Server; the
Client only draws what the Server tells it.

## User Stories

1. As a Player, I want a removed Voxel to leave an item behind, so that digging
   produces something I can collect.
2. As a Player, I want a dropped item to fall and come to rest on the ground, so
   that it behaves like a thing in the World.
3. As a Player, I want a dropped item to stop on a ledge, in a cave or in
   water-free air above a stair, so that it never sinks into terrain.
4. As a Player, I want to pick an item up by walking near it, so that I need no
   extra key.
5. As a Player, I want an item I just dropped or removed to wait a moment before
   it can be picked up, so that I do not collect it by accident while breaking
   the Voxel.
6. As a Player, I want nearby items of the same kind to merge into one stack, so
   that a mined pile is not hundreds of spinning cubes.
7. As a Player, I want an uncollected item to disappear after a long while, so
   that a long-lived World does not fill with litter.
8. As a Player, I want to see how many of each item I carry, so that I know
   pickup worked.
9. As a Player, I want dropped items and my carried count to survive Save &
   Quit, so that closing the game does not lose them.
10. As a Player, I want an item that fell into a Chunk that is not loaded to
    still exist when I come back, so that walking away does not destroy it.
11. As a Game author, I want to declare items by name in my Declaration, so that
    I decide what exists.
12. As a Game author, I want to say which Material drops which item, so that a
    removed Voxel yields what makes sense for my Game.
13. As a Game author, I want an item's size in metres, so that it looks right at
    any Voxel size.
14. As a Mod author, I want a data-only Mod to declare items too, so that a
    shared Mod can ship its items beside its Materials.
15. As a Mod author, I want a refusal that names my Mod and field when an item
    is wrong, so that I can fix it.
16. As a Mod author on contract line 0.1 or 0.2, I want my Mod to keep loading
    with no items, so that nothing I shipped breaks.
17. As an engine developer, I want the entity simulation to run on the Server's
    fixed logical step, so that equal state and equal steps give equal results.
18. As an engine developer, I want the item simulation testable without Bevy, so
    that it is benchmarkable like everything else in `tesserae-core`.
19. As an engine developer, I want the Client to hold no entity state of its own,
    so that single player does not hide a Server-authority bug (ADR-0002).
20. As an engine developer, I want the number of live dropped items bounded, so
    that a mined mountain cannot stall a tick or bloat a save.
21. As a later-step author (inventory, mobs), I want entity storage that is
    additive to the save and separate from the World header, so that I can add
    kinds of entity without a format change to what exists.
22. As a Player, I want undoing my removal not to make the Server misbehave, so
    that undo, redo and drops stay consistent.
23. As a Player on a World saved before this step, I want it to be refused or
    opened exactly as every other build change is, not corrupted.

## Implementation Decisions

**Vocabulary** (added to `GLOSSARY.md`):

- **Item** — a thing a Game or Mod declares by name. Not a Voxel, not a Tool.
- **Dropped item** — an Entity of kind item: one Item with a count, at a
  position in metres, falling or at rest.
- **Entity** — a Server-owned thing with a position in metres that is not a
  Voxel, a fluid or a Player. This step builds one kind; mobs (step 22) add
  more.
- **Carried** — the count of each Item a Player holds after Pickup.
- **Pickup** is redefined: a Dropped item entering a Player's Carried, by
  nearness. Still distinct from Use and Sample.

**Item definitions (the Declaration).**

- `declaration` gains one field, `items: list<item-def>`, appended last, on the
  unreleased 0.3 contract. `item-def` has a local `name`, a `size-m` (edge of
  the item's cube in metres, bounded), an optional `material` (a solid
  Material's full name `<mod id>:<name>`) and a display `color`.
- `material` is the single link between a Voxel and a drop (the reverse of
  Minecraft, Luanti and Vintage Story, which put drops on the block; chosen so
  `material-def` and every guest's Material rows stay unchanged, and revisited
  with counts, chances and tool requirements at steps 15–16): removing a Voxel of
  that Material yields one of that Item. No field is added to `material-def`.
  Two Items naming one Material fails the load.
- Allowed from any Mod, not Game-only: items are shared content like
  Materials. Items resolve after every Mod's Declaration, like `fluids`.
- Item ids are registration order across the load, held by the Server in its
  own resource, not the Registry: saved data uses names, so no id is ever
  replayed.
- Lines 0.1 and 0.2 adapt to an empty `items`. The data Declaration gets
  `[[items]]` with kebab-case fields, unknown fields refused, as elsewhere.
- Bounds: at most 64 items per Mod and 256 per World; `size-m` between 0.05 and
  0.5.
- A starter's `wit/` copy and the authoring guide's schema are updated;
  `docs/contract-compatibility.md` gains one sentence recording the amendment
  and that older lines load with none.
- Not added: a Command to spawn items. A Mod cannot make a drop except by
  declaring which Material drops it (a gap recorded in Out of Scope).

**Entities (`tesserae-core`, no Bevy).**

- The store is a plain struct with no Bevy type (code name `WorldEntity`, to
  stay clear of Bevy's `Entity`); each entity carries its Chunk coordinate so
  step 22 can shard storage per Chunk as other engines do. Today it is one
  global bounded store.
- A new module holds the entity store and its stepping: stable monotonic
  `EntityId`, a bounded set of Dropped items, and `step` over a
  `VoxelVolume`. Motion reuses the AABB sweep in `collide`; gravity and every
  size are in metres and seconds, never Voxels.
- The Server advances them on its 1/64 s logical step, bounded per tick like
  fluids (ADR-0014), so equal state and equal steps give equal positions. A
  pop-out velocity at spawn is derived from the id and the step, not a random
  generator, so no PRNG state needs saving.
- An item rests when a downward sweep is blocked and it has no horizontal
  speed. It falls again if the support under it is removed: resting items are
  re-checked when a Chunk they stand in changes.
- Items in a Chunk that is not held do not move and do not age; they stay in the
  store and in the save. Simulation resumes when the Chunk is admitted. This
  stops an item sinking through ground that has not generated.
- Merge: a resting item merges into another at rest of the same Item within
  0.5 m (an engine constant), summing counts, keeping the older id.
- Age is in logical steps and saved as steps remaining, never wall time. At 300
  s of simulated time an uncollected item despawns.
- The live set is capped at 2048. Spawning past the cap despawns the oldest.
  The cap, merge radius and despawn time are engine constants, not
  declarations; a Game that needs them tunable is a later contract line.

**Server (`tesserae`).**

- A new plugin, `server::entities`, registers the step system, spawn-on-remove,
  pickup and the replication messages, and is added to the Server with one
  `add_plugins` statement of its own. No other Server plugin is edited except
  to emit the event below.
- Spawn: a deliberate single Remove of a Voxel whose Material has an Item emits
  a removal event from `edit`; the entities plugin spawns the drop at the
  Voxel's centre in metres. Not spawned by undo, redo, Layout removal, fluid or
  fire, or a Mod's Commands. Undoing a removal leaves its drop where it is; the
  duplication is accepted until step 15 composes inventory with the undo
  journal (DESIGN).
- Pickup: each tick, an item past its pickup delay (0.5 s) within 1.5 m of the
  Player's body box is removed and added to the Player's Carried. Pickup is not
  a Use and enters no history.
- Carried is a `BTreeMap<item name, u32>` on the Server's Player, the smallest
  sink that makes Pickup observable. It is deliberately not an inventory (no
  slots, no stack limit, no drop-out, no transactions); step 15 replaces it.
  Counts saturate rather than wrap.
- Replication: the Server writes a message per change (spawned, moved, removed)
  and a Carried message, each tick like `UseOutcome`, so a frame that runs no
  tick drops nothing. The Client reads them. The message shape is the part step
  22 designs against transport and is kept to plain data.
- Invariant check: no Client system takes `ResMut` of any entity store, and the
  Client never reads World state to find one.

**Client (`tesserae`).**

- A view-only mirror keyed by `EntityId`, drawn as a small cube of `size-m`
  using the Item's Material appearance and texture when it has a Material, its
  `color` otherwise, slowly spinning and bobbing. Cosmetic motion is Client-side
  only and never feeds back.
- UI, agreed against a mockup: a drop is a small spinning, bobbing cube with a
  soft contact shadow; Carried is one line in the F3 diagnostics plus a bottom
  right toast (`+1 terra:stone`) that fades after about 1.5 s. Nothing outside
  F3 shows Carried, and there are no hotbar changes, names over drops or pull
  animation. The panel is step 15.

**Save (additive, separate from the World header).**

- `save::Commit` gains one field, `entities`, with `serde(default)`, holding
  saved Dropped items: item name, count, position in metres, velocity, resting
  flag, steps remaining, and the next `EntityId`. Its types, validation and
  restore live in their own `save` submodule. `Commit::compatible` gains one
  call into it. Nothing in `format`, `generator`, `parameters`, `content` or
  `player.eye_m` changes. Format stays 1 and the generator version is untouched
  here; step 13 pins the generator version in its own field.
- Carried is saved as a Player extension, `carried_v1`, using the existing
  extension map; older engines already refuse unknown extensions.
- Names are resolved against the Registry on load, like Materials; an unknown
  Item name or non-finite number is a corrupt commit.
- Save dirtiness includes an entity revision that bumps on spawn, despawn,
  merge, pickup and coming to rest. A mid-fall position is saved as it is and
  resumes.
- The entity set is saved whole in every commit. The 256 MiB cap and the
  no-garbage-collection note in `docs/world-persistence.md` still apply; the
  2048 cap bounds the cost, recorded in that document.
- A World saved by an earlier build opens as before with no entities where the
  build identity allows it, and is refused where it does not, as today.

**Shared surfaces with step 13 (Biomes), decided here.**

- Save: one added `Commit.entities` field in its own submodule, one added
  extension key; no edit to the World header or `Generation`. A textual
  conflict in the `Commit` literal in persistence is expected and trivial;
  whichever branch merges second resolves it.
- Contract: one appended `items` field and one record; no reordering. Step 13's
  biome declarations append their own field; the two are independent.
  `docs/contract-compatibility.md` and the authoring guide's schema table are
  edited by appending a row or sentence, not rewriting.
- Wiring: the plugin is added by its own line; the Server tick is not edited.

**Invariants.** No Client system mutates entity state. Mods reach items only
through the Declaration, in the WIT contract; no host call is per item. All
motion, reach, size and radii are metres and seconds.

## Testing Decisions

- A good test drives external behaviour at the highest seam: given a World and
  Declaration, what exists after N logical steps and edits. No test asserts on
  private fields.
- **Seam 1: `tesserae-core` entity module**, headless: fall, rest, wall slide,
  support removal, merge, despawn, cap, determinism (two runs equal), frozen in
  an unheld Chunk. Prior art: `flow` and `collide` tests.
- **Seam 2: the headless Server app**, driving real messages: Remove yields a
  drop, no drop for undo/Layout/Commands, Pickup moves a drop into Carried,
  delay respected, replication messages emitted, Client mirror never writes.
  Prior art: `server::edit` and `server::tool` tests.
- **Seam 3: save round trip** through `core::save` and `server::persistence`:
  drops and Carried survive Save & Quit and restart, unknown Item name refused,
  entity-free commit still opens. Prior art: `persistence/tests.rs`.
- **Seam 4: Declaration loading** in `tesserae-mods`: valid item, every
  refusal naming Mod and field, older lines adapt to none. Prior art:
  `tests/declare.rs` and `tests/loading.rs`.
- No GPU or screenshot test. The Client drawing is checked once by the user,
  with `TESSERAE_SCREENSHOT_OFFSCREEN=1`; captures are review-only and deleted.
- A bench in `tesserae-core` for stepping 2048 items, and a before/after of an
  idle tick, interleaved on thread CPU time against main.

## Out of Scope

- Inventory, slots, stack limits, dropping from a hand, crafting (steps 15–16).
- Placing an Item as a Voxel; item use. Tools remain as they are.
- Mobs, animals, NPCs and any entity with behaviour (steps 21–23). The store is
  a place for them; their kinds, AI hooks and transport design are not here.
- A Command or host function for a Mod to spawn or query items. Recorded as a
  contract gap for the step that needs it.
- Per-item or per-Material drop counts, tables, chances, and tool requirements.
- Physics between items, fluid drift and burning items.
- Sound (step 14) and any network transport.
- Changing the World header, the generator version or worldgen (step 13).

## Further Notes

- Roadmap step 12 in `DESIGN.md` moves to Done at the end of the last ticket.
- Mods in `mods/` and Terra gain a few items so the feature is demonstrable;
  that is the last ticket, not a prerequisite for the engine work.
- A save made by this build holds `entities` and `carried_v1`; an older engine
  refuses it through its existing unknown-field and unknown-extension checks.
