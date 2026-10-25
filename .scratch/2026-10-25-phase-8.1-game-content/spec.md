# Step 8.1: water is Game content, and the player places anything declared

Status: ready-for-agent

Drafted 2026-10-25 from `feat/phase-8.1-game-content` (first named `feat/phase-8.1-water-is-game-content`) at 8f43e26
(phase 8, PR #19, merged as 9992e8e). Approved by the user on 2026-10-25. A
push or PR is separate authorization.

Decided with the user on 2026-10-25: the picker menu is engine Client UI; id 7 is
retired and a Mod's Materials still start at 8; a World with no Game is dry
and `cargo dev` loads the in-repo demo by default; the contract gets a new 0.3
line. Placing a Mod's solid Materials and a selection menu were added to the
scope on the same day. Data-only Mods are a later session once this lands.

## Problem Statement

Tesserae ships no gameplay: the engine owns how fluids flow, waterlog, react,
heat, burn and put fire out, and a Game's Declaration says which Materials do
it. Lava, mud and wood already work that way. Water does not. It is built-in
Material 7 with a built-in fluid row that extinguishes, reserved in phase 5 for
generated lakes before fluids existed. So the engine decides that every World
has water, how far it reaches and that it puts out fire. A Game cannot leave it
out, retune it or have only lava lakes. The demo's lava-and-water reaction has
to name `core:water` instead of its own Material.

The player is also limited to what the engine built in. The hotbar is a fixed
list of built-in solids plus water, and the fluid tool places water only. Lava,
mud, fire and every solid a Game declares, such as wood, brick or a lamp, reach
the World only through Mod Commands. A Game author cannot try their own
content by hand, and a player cannot use it.

No saves exist yet, so dropping a permanent built-in id costs little now and a
migration later.

## Solution

Water becomes the demo Game's Material, declared and tuned like lava. The
built-in Materials are air, stone, dirt, grass, lantern, glass and leaves, ids 0
to 6. Id 7 is retired and never registered again, and a Mod's Materials still
start at 8. The built-in fluid row goes with it, leaving the built-in fluid
table empty.

Generated lakes fill with the fluid the Game names in its World parameters,
any fluid it declared. A World with no Game has no fluid, no lakes and nothing
that burns: the engine on its own is dry. A `dev` build loads the in-repo demo
by default when it has been built, so `cargo dev` still opens a World with
water, lava and fire. When the demo has not been built, it opens dry and says
how to build it.

The player places anything the Game declared. The hotbar's slots hold one of
three things: a solid (a Material in a Shape), a source of a declared fluid, or
fire. An engine menu lists every declared solid Material, every Shape, every
declared fluid and fire, and picking an entry puts it in the selected slot.
Fire ignites declared fuel and puts burning out. Every action goes to the
Server as intent and is decided by the Server's existing rules. A Mod on the
new 0.3 contract line sees a player's ignitions and put-outs as it already sees
their solid and source Edits.

## User Stories

1. As a Game author, I want to declare water as my own Material with my own
   fluid row, so that its reach, rates, waterlogging and extinguishing are my
   Game's choices.
2. As a Game author, I want to name the fluid my lakes are made of, so that my
   World can have water lakes, lava lakes or none.
3. As a Game author, I want to leave water out entirely, so that a dry or
   lava-only World is a choice I can make.
4. As a Game author, I want my reactions to name my own water, so that "lava
   touching water makes stone" is my rule in my namespace.
5. As a Game author, I want a lake fluid that does not exist, or is not a
   fluid, to fail the load with a message naming the field, so that a typo
   does not give me a silently dry World.
6. As a Game author, I want the default World parameters the engine hands me to
   have no lakes, so that I opt in to standing fluid rather than out of it.
7. As a Game author on the 0.2 line, I want my Game to keep loading unchanged,
   so that I am not forced to rebuild for this release.
8. As a Game author on the 0.2 line who asked for standing water, I want a
   warning saying my World is dry and why, so that the missing lakes are
   explained.
9. As a Mod author on the 0.1 line, I want my component to keep loading
   unchanged, with the same Material ids it saw before, so that the adapter's
   promise holds.
10. As a Mod author, I want `core:water` absent from the registered names
    rather than mapped to something else, so that a lookup for it fails
    plainly.
11. As a Mod author on 0.3, I want to see a player's ignitions and put-outs in
    the Edit history, so that my Game can react to fire the player started.
12. As a Mod author on 0.2 or 0.1, I want the new Edit kinds left out of what I
    see, so that my closed variants are never handed a case they do not have.
13. As a player, I want to place a source of any fluid the Game declared,
    water, lava or mud, so that I can try every fluid by hand.
14. As a player, I want to remove a source of any fluid, so that I can stop a
    stream I started.
15. As a player, I want to set declared fuel alight, so that I can start a fire
    without a Mod scripting it.
16. As a player, I want to put out a burning Voxel, so that I can stop a fire.
17. As a player, I want igniting wet fuel, or anything that does not burn, to
    be refused with the existing notice, so that I know why nothing happened.
18. As a player, I want to place any solid Material the Game declared, in any
    registered Shape, so that wood, brick and a lamp are mine to build with.
19. As a player, I want a menu listing everything I can place, with each
    entry's name and placeholder colour, so that I can find it without knowing
    ids.
20. As a player, I want picking an entry in the menu to put it in the hotbar
    slot I have selected, so that keys 1 to 9 stay a fast way to switch.
21. As a player, I want the menu to free the cursor while it is open and grab it
    back when it closes, so that I can click entries and then return to
    building.
22. As a player, I want the hotbar to start with useful defaults: the built-in
    solids, the Game's lake fluid or first declared fluid, and fire when
    anything burns, so that a new World is playable without opening the menu.
23. As a player, I want left and right click to mean what the held slot is,
    break and place for a solid, remove and place for a fluid, put out and
    ignite for fire, so that one modifier key is not needed for every tool.
24. As a player, I want the HUD to name what each slot holds and what the
    buttons do with it, so that I always know what a click will do.
25. As a player in a World with no Game, I want the menu to offer only the
    built-in solids, so that it never offers something the Server would refuse
    for being unknown.
26. As a developer, I want `cargo dev` to open the demo World by default when
    it has been built, so that water, lava and fire are on screen without
    flags.
27. As a developer, I want `cargo dev` without a built demo to open a dry World
    and say to run the guest build, so that a missing artifact is not a silent
    change.
28. As a developer, I want an explicit mods directory to replace the default
    demo, so that I can run another Game or none.
29. As a developer, I want a release build never to load the in-repo demo on
    its own, so that the default exists only for development.
30. As a developer, I want the in-repo demo trusted automatically in a `dev`
    build only, so that each guest rebuild does not prompt while a shipped
    binary keeps the full Trust check.
31. As a developer, I want `TESSERAE_WATER_LEVEL_M` to move the Game's lakes,
    and to warn and do nothing when the Game names no lake fluid, so that the
    lake override still works.
32. As a developer, I want the translucent gallery to draw its pool with the
    Game's lake fluid, and to leave the pool out when there is none, so that
    the blended-pass check still runs with the demo loaded.
33. As an engine developer, I want tests and benches to register a fixture
    fluid of their own, with today's water numbers, so that nothing in the
    engine's test suite depends on a built-in fluid.
34. As an engine developer, I want the level-1 waterline and the coarse
    under-water light to read the Game's lake fluid, so that LOD treats lava
    lakes and water lakes alike.
35. As an engine developer, I want smoothing to yield to the lake fluid, so
    that an underwater step stays a step whatever the lake is made of.
36. As an engine developer, I want the spawn point to clear the Game's lakes,
    so that the player never spawns under standing fluid.
37. As a reviewer, I want an ADR that amends ADR-0009's list of permanent
    built-ins and records id 7 as retired, so that the save-data promise is
    stated where it is enforced.
38. As a reviewer, I want CONTEXT.md, architecture.md and DESIGN.md to stop
    saying water is built in, so that the docs do not keep the exception after
    the code drops it.
39. As a reviewer, I want the phase 7 drain cap and the Wasmtime, WASI and
    Cranelift pins unchanged, so that this phase moves no measured budget or
    security fix.

## Implementation Decisions

1. **Built-in Materials.** The built-ins are ids 0 to 6: air, stone, dirt,
   grass, lantern, glass and leaves. Id 7 is retired. The Material table
   reports it unknown, the Registry never lists it, and a Mod's Materials are
   still numbered from 8 in registration order. An unknown id in a Chunk is
   still drawn as the opaque stand-in ADR-0009 describes. The built-in Material
   constant and the built-in water row are deleted, not kept as aliases.
2. **Built-in fluids.** The built-in fluid table is empty. Every fluid row comes
   from a Declaration and is validated the same way. Nothing in the engine
   assumes some fluid extinguishes.
3. **ADR-0015** records decisions 1 and 2 and amends ADR-0009's list of
   permanent ids. It also notes what this changes in ADR-0012 (the 0.3 line,
   decision 6) and ADR-0013 (generated lake water is the Game's fluid). The ADR
   describes the approach in Tesserae's own terms and cites no other engine.
4. **Lakes name their fluid.** The engine's World parameters for standing fluid
   carry a level in metres and the lake's fluid Material. Generation fills open
   air below the level with packed cubes of that Material, which Occupants
   already reads as sources. The engine's default World parameters have no
   lakes. Validation resolves the name after every Declaration's behaviour rows,
   and fails the load when the name is unknown, names a solid or has no fluid
   row. Any declared fluid may fill a lake: hot, opaque or not waterlogging.
5. **Everything derived from the lake reads its Material.** Slope smoothing
   yields to it. The level-1 waterline treats only packed lake-fluid cubes below
   the level as generated, and any other fluid there as dynamic. Coarse
   under-water light uses the lake fluid's own dimming. The spawn point clears
   the level. `TESSERAE_WATER_LEVEL_M` moves the level when a lake fluid is
   named, and logs a warning and changes nothing when none is.
6. **Contract line 0.3.** ADR-0012 already says a closed record or variant that
   grows needs a new line. Two grow:
   - The standing-fluid record gains the lake fluid's full name.
   - The player Edit kind gains an ignition and a put-out, each carrying the
     fuel's Material id.

   Nothing else changes. 0.3 is the line the host speaks natively. The 0.2 and
   0.1 directories stay frozen.
7. **0.2 and 0.1 keep loading through adapters.** A 0.2 component is carried to
   0.3 by a new one-step adapter. A 0.1 component goes through the existing 0.1
   adapter and then the 0.2 one, so each adapter knows one step only.
   - Downward, the Edit history drops ignitions and put-outs. That matches how
     0.1 already loses Edits that leave the packed Voxel unchanged.
   - Upward, a 0.2 or 0.1 Game's standing-fluid record has no fluid to name. It
     is dropped with a logged warning, and that World is dry.
   - A 0.2 or 0.1 Mod's rows or Commands that name `core:water` fail the way any
     unknown name does.
   - Every 0.1 fixture passes unchanged, its id assertions included. The 0.2
     `fluids` fixture renames its reaction partner to a fluid it declares itself.
8. **The demo declares water.** The demo moves to 0.3 and declares
   `demo:water`: translucent, dimming 2, hiding the same Material, drawn
   blended and not solid. Its fluid row carries today's numbers: reach 7 m,
   4 m/s sideways, 8 m/s down, 4 m/s recession, 0.5 m reset drop, waterlogging
   and extinguishing. It also declares today's placeholder colour, the
   lava-and-water reaction with its own water, and lakes of `demo:water` at a
   level that leaves its scenario stage dry. Every scenario plays out as it
   does today at 1.0, 0.5 and 0.25 m.
9. **A World with no Game is dry.** It has no lakes and no fluid rows. The
   engine's own placeholder palette has no water entry: colour for a declared
   fluid comes from its Game's appearance, as lava's does.
10. **`cargo dev` loads the demo by default.** In a `dev` build only, when no
    mods directory is given on the command line or in the environment and the
    demo's built output exists in the repository, the loader uses it.
    - That directory's Mods are trusted for the run without a prompt and
      without writing a Trust record.
    - When the output does not exist, the World opens dry, and the log says to
      run the guest build script.
    - An explicit directory always wins.
    - A release build's default stays the user's data directory, with the full
      Trust check.
11. **The Server publishes a content catalogue to the Client.** It is plain
    content beside the existing palette message. Each registered Material has
    its full name and id, whether it is solid, whether it has a fluid row and
    whether it has a combustion row. Each registered Shape has its full name, id
    and whether turning it changes anything (not a full cube, not connected).
    The catalogue is read-only content, not World state: the Client still never
    reads a Chunk (ADR-0002).
12. **The hotbar is data.** It has nine slots on keys 1 to 9 in every build. A
    slot holds a solid (Material, Shape, Orientation), a fluid source (a
    Material), fire, or nothing.
    - **Defaults:** stone cube, stone slab, stone stair, stone post, lantern,
      glass and leaves. Then the Game's lake fluid, or else its first declared
      fluid. Then fire when any combustion row exists. Any slot left over is
      empty.
    - The dev Server's reference fence is no longer a slot of its own: it is a
      registered Shape like any other, picked from the menu.
    - Q and E still turn and flip a held solid.
13. **Clicks follow the held slot, and the Alt modifier goes.**

    | Held slot  | Left click         | Right click       |
    | ---------- | ------------------ | ----------------- |
    | Solid      | break              | place             |
    | Fluid      | remove a source    | place a source    |
    | Fire       | put out            | ignite            |
    | Empty      | break              | nothing           |

    The HUD names the held slot and these two actions.
14. **The picker menu.** It is engine Client UI, opened and closed with Tab. While
    it is open the cursor is free and mouse look and clicks to edit are off;
    closing it grabs the cursor again. It lists, from the catalogue:
    - solid Materials, then the Shapes for the chosen Material;
    - fluids;
    - fire, when anything burns.

    Each entry shows its full name and its placeholder colour. Choosing one puts
    it in the selected slot. Escape closes the menu too. The Game decides
    nothing about the menu.
15. **Ignite and put out on the wire.** The edit intent gains two actions,
    ignite and put out. Both carry the same ray in metres as every other intent
    and go through the same origin check, reach and queue. Both strike the
    first solid on the ray, and the Server applies the rules Mod Commands
    already use:
    - Ignition needs declared fuel that is neither burning nor wet, and lights
      it with its full fuel.
    - A put-out needs a burning solid.
    - Refusals reuse today's reasons: not flammable, already burning, too wet to
      burn, nothing burning there.
    - A successful one is reported to 0.3 Mods as decision 6's Edit kind, with
      the fuel's Material.
16. **Placing a declared solid** uses the existing place action unchanged. The
    Server already vets any registered Material and Shape. A Mod already sees
    the result as a solid Edit.
17. **A fixture fluid for tests and benches.** `tesserae-core` exports, beside
    its existing deterministic test terrain, a way to register a fixture water
    Material and fluid row, with today's water numbers and Material properties,
    into a set of tables. Tests and benches in all three crates use it instead
    of the built-in constant. It is a fixture: nothing in the running engine
    calls it.
18. **Unchanged:**
    - the phase 7 drain cap of two Voxels a tick;
    - Wasmtime and WASI at 49.0.4, Cranelift at 0.136.4;
    - every simulation bound in ADR-0014;
    - the Overlay and Occupants encoding in ADR-0013.

## Testing Decisions

A good test drives a seam from outside and asserts what a player, a Mod or the
World would see: the Occupants at a position, the ids and names a Mod is
handed, the rejection a Client receives, the intent a click sends. It never
asserts on private structure. Wall-clock limits in any test are hang detectors
with wide margins, never budgets: CI builds dependencies at opt-level 0 on a
runner about six times slower than the development Mac.

These are the seams, highest first. No new seam is introduced.

1. **The Mod load path:** components and Manifests in, then the tables, the
   Registry, the World parameters and load errors out. This is where the existing
   contract-line and behaviour tests and the guest fixtures already sit. It
   covers:
   - the 0.3 Declaration with a lake fluid;
   - unknown, solid and rowless lake names failing the load;
   - a 0.2 Game's water dropped with a warning;
   - the 0.1 fixtures passing unchanged with Mod ids from 8;
   - `core:water` unknown on every line;
   - ignitions and put-outs reaching 0.3 Mods and projected away for 0.2 and 0.1.
2. **The Server's edit path:** an edit intent in, then World Occupants and the
   rejection message out. These are the existing Server edit tests. They cover
   ignition, put-out, wet fuel, non-fuel, already burning, nothing burning, a
   source of each declared fluid, and placing a Mod's solid.
3. **World generation:** parameters and tables in, Chunk Voxels out, as in the
   existing generation tests. It covers lakes of a named fluid, a lava lake, no
   lakes by default and smoothing yielding to the lake.
4. **Coarse derivation and light:** the existing level-1 derivation tests, with
   the waterline and coarse under-water light reading the lake fluid.
5. **The Client's input:** key and button input with a catalogue in, then the
   sent edit intents and the HUD text out. These are the existing hotbar tests
   that run the Client systems headlessly. They cover the defaults, picking from
   the menu into a slot, clicks per slot kind and the menu holding back edits
   while it is open.
6. **Loader options:** arguments, environment and paths in, then the chosen
   directory and Trust mode out, as the existing option resolution tests do.
   They cover the dev demo default, an explicit directory winning and release
   unchanged.
7. **The demo end to end:** the existing demo runtime tests load the built
   demo, now on 0.3, and run each scenario at the three Voxel sizes.

Benches keep their fixtures equal: they register the fixture fluid, so their
numbers stay comparable with main. Any bench comparison follows the interleaved
CPU-time method from phase 8.

## Out of Scope

- **Data-only Mods**, a Declaration from a data file with no component. They are
  agreed as their own session once this lands.
- **A guest SDK crate.** It would amend ADR-0004's boundary for the demo.
- **Menu content or layout from a Mod.** The menu is engine UI. A Game ordering
  or hiding entries is a later contract decision.
- **An inventory, item counts or creative and survival modes.** The hotbar and
  the menu stay a development stopgap that happens to reach all content.
- **Save data, Registry tombstones beyond id 7, and migration of a missing Mod.**
- **Water shading and Look packs**, step 9.
- **Dropping the 0.1 line.**
- **Network transport.** The intent stays an in-process message, as today.

## Further Notes

- **Build hygiene.** Every cargo command runs with `RUSTFLAGS="-D warnings"`,
  uses the `--workspace --features tesserae/dev` forms and runs under
  `caffeinate -i`. Run `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci`
  before each implementation commit. Never run Prettier on `.scratch`.
- **Rebuilding guests.** Guests and the demo are rebuilt with the guest build
  script after any WIT or `games/` change. CI already builds them.
- **Prior art in other engines** may be cited as behaviour in our own words with
  links. No code or long passages are copied. The ADR, these tickets and the
  commits name no other open-source voxel engine that keeps its content in a
  Game.
- **Docs to update with the code:** CONTEXT.md (Fluid, Source, Water level),
  architecture.md, DESIGN.md (step 8's water wording, the open items' "places
  water only", and a step 8.1 entry), README.md and CONTRIBUTING.md (controls,
  running the demo), and the WIT comments.
