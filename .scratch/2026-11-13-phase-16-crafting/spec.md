Status: done

# Phase 16: Crafting, Stations, Containers and Doors

DESIGN.md roadmap step 16: Recipe schema, Station interaction, Use on Voxels
(Doors, Containers) and the UI host. Depends on step 15 (Inventory, done). The
Engine adds the mechanisms; a Game or Mod supplies Recipes, Stations, Containers
and Doors.

Vocabulary: **Recipe**, **Craft**, **Usable Voxel**, **Station**, **Container**
and **Door** are defined in [GLOSSARY.md](../../GLOSSARY.md), and **Use** is
amended there. The decision records are
[ADR-0026](../../docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md)
and the 2026-11-13 amendment to
[ADR-0023](../../docs/adr/0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md).
The approved look is the step 16 section of [mockup/](../../mockup/README.md).

## Problem Statement

A Survival player gathers Rock, Wood and Dirt and can only place them back.
Nothing turns what they gather into anything else, so a Game cannot make one
Item cost another, and there is no reason to gather more than one kind of thing.
A player cannot store anything outside their 36 slots: a full Inventory means
throwing Items away. There are no doors, so a shelter is either sealed or open
to the wind. Use answers a held Tool or eats; aimed at a Voxel with nothing in
hand, it does nothing.

A Game author cannot declare a Recipe, a workbench, a chest or a door. The
contract has no rows for them, and per ADR-0001 the answer cannot be a Mod
callback on every Use.

## Solution

A Game or Mod declares Recipes as data: the Items spent, the Item made, and
where it is crafted, `"hand"` or a Station's tag. It declares Usable Voxels as
data too: a Material is a Door (a closed and an open Material), a Container (a
slot count) or a Station (a tag). The Server runs each kind natively; no Mod is
called when a Voxel is used.

Pressing I opens the Inventory with a Crafting tab: the hand Recipes, craftable
anywhere. Aiming at a Usable Voxel shows a prompt under the reticle, and Use (G)
acts on it: a Door opens or closes, a Chest opens its slots beside the
Inventory, and a Workbench opens the Recipes that need it. Walking out of reach
closes the pane. Crafting is instant, takes from the Inventory and gives to it;
what does not fit drops at the Player's feet.

Terra ships Wood into a Workbench by hand, then a Chest and a Door at the
Workbench, with sounds for doors, chests and crafting.

The item details panel in the Inventory window becomes compact and previews each
Shape.

## User Stories

Hand crafting

1. As a Survival player, I want a Crafting tab in my Inventory window, so that I
   can turn what I gathered into something else without placing a block first.
2. As a Creative player, I want the Crafting tab beside the Catalogue, so that I
   can switch between taking Items and making them.
3. As a player, I want every hand Recipe listed with its output and inputs, so
   that I can see what I could make.
4. As a player, I want a Recipe I cannot afford dimmed with its missing inputs
   tinted, so that I know what to gather next.
5. As a player, I want to select a Recipe and see what I have against what it
   needs, so that I can decide before I spend anything.
6. As a player, I want Craft 1 and Craft N buttons, N being as many as I can
   afford up to one output Stack, so that I can make one or a batch.
7. As a player, I want double-click to craft one and Shift-click to craft N, so
   that frequent crafting is quick.
8. As a player, I want to search Recipes by output name, so that a long list
   stays usable.
9. As a player, I want crafted Items to go straight into my Inventory, so that I
   do not have to collect them.
10. As a player, I want what does not fit to drop at my feet with a notice, so
    that a full Inventory never destroys a craft.
11. As a player, I want crafting to be instant, so that it never interrupts
    building.
12. As a player, I want undo (Z) to leave crafts alone, so that undoing a
    placement never unmakes my tools.
13. As a player, I want a craft refused when I lack the inputs, so that Items
    are never minted.

Use on Voxels

14. As a player, I want a prompt under the reticle when I aim at a Usable Voxel,
    naming what Use would do, so that I know a door or chest is interactive.
15. As a player, I want the prompt to say "Out of reach" before I press, so that
    I walk closer instead of guessing.
16. As a player, I want Use to act on the aimed Voxel even with food in hand, so
    that opening a door never eats my bread.
17. As a player holding a Tool, I want Use to stay the Tool's action, so that
    measuring next to a door still measures.
18. As a player, I want Use to keep eating when I aim at nothing usable, so that
    eating still works as before.
19. As a player, I want the Use binding named "Use", so that its label says what
    it does.

Doors

20. As a player, I want Use on a Door to open or close it, so that I can walk
    through and shut the wind out.
21. As a player, I want a two-Voxel door to open and close as one, so that half
    a door never stands open.
22. As a player, I want a Door to keep its Shape and Orientation when it
    toggles, so that it opens in place.
23. As a player, I want closing a Door on myself refused, so that I never get
    stuck inside it.
24. As a player, I want breaking an open Door to give the same Item as a closed
    one, so that I always get my door back.
25. As a player, I want opening and closing doors kept out of my undo history,
    so that Z undoes my building, not my walking.
26. As a player, I want undo of a door's placement to work whether it is open or
    closed, so that toggling it never blocks undo.
27. As a player, I want doors to stay as I left them after I reopen the World,
    so that my house is as I left it.

Stations

28. As a player, I want Use on a Workbench to open its Recipes beside my
    Inventory, so that I can make things that need a bench.
29. As a player, I want the Station pane to list only that Station's Recipes, so
    that I can tell bench Recipes from hand ones.
30. As a player, I want a Station Recipe craftable only while I am at that
    Station, so that building a Workbench means something.
31. As a player, I want the Station pane to close with a notice when I walk out
    of reach, so that I know why it went away.
32. As a player, I want any Material that serves the same Station tag to open
    the same Recipes, so that a Mod's oak workbench works like Terra's.

Containers

33. As a player, I want Use on a Chest to open its slots beside my Inventory, so
    that I can store what I do not carry.
34. As a player, I want to move Stacks between the Chest and my Inventory by
    click, drag and Shift-click, so that storing feels like my Inventory.
35. As a player, I want Take all and Sort on the Chest, so that emptying and
    tidying it is one click.
36. As a player, I want the Chest's contents to stay when I leave and to survive
    reopening the World, so that storage is real.
37. As a player, I want breaking a Chest that holds Stacks refused with "Empty
    the Chest first.", so that I never scatter a hoard by accident.
38. As a player, I want undo and redo refused when they would remove a full
    Chest, so that the same rule holds through history.
39. As a player, I want a Chest's contents to spill as Dropped items when
    something other than me replaces it, so that a Mod or a fire never deletes
    them.
40. As a player, I want the Chest pane to close when I walk out of reach or the
    Chest changes, so that I never edit a Chest I am not at.
41. As a future multiplayer player, I want two players at one Chest to see the
    same contents, so that sharing storage is safe.

Item details

42. As a player, I want the item details panel to fit without scrolling, so that
    every action is visible.
43. As a player, I want each Shape button to preview the Item in that Shape, so
    that I choose by sight.
44. As a player, I want the header icon to follow the Shape I choose, so that I
    see what I will place.

Game and Mod authors

45. As a Game author, I want to declare Recipes as data with `station = "hand"`
    or a Station tag, so that a data-only Game can have crafting.
46. As a Game author, I want `station` required and `hand` reserved, so that a
    forgotten field is an error, not a hand Recipe.
47. As a Mod author, I want to add Recipes for another Mod's Items and Stations,
    resolved after every Mod declares, so that Mods compose.
48. As a Game author, I want an unknown Item or Station tag to be a load error
    naming the Recipe, so that typos never ship.
49. As a Game author, I want to declare a Material a Door, Container or Station
    in one row, so that usable blocks are as easy as Materials.
50. As a Game author, I want a Material that is two kinds refused at load, so
    that Use never has two meanings.
51. As a Game author, I want to declare a Container's slot count from 1 to 54,
    so that a pouch and a vault can differ.
52. As a Game author, I want Causes for door, container and craft sounds, so
    that I can give them sounds like any other Cause.
53. As a Mod author, I want the authoring guide to show hand and Station Recipes
    side by side, so that I understand the difference.
54. As a Mod author, I want the Example Game to declare one Recipe, Door,
    Container and Station, so that I can copy a working row.

## Implementation Decisions

**Contract: line 0.3 is amended in place** (it is unreleased; see the "Amended
before release" section of the contract-compatibility doc). The Declaration
gains two lists, and the `sound-cause` enum grows.

- `recipes`, from any Mod, at most 256 per Mod and 1024 per World. A row has 1
  to 9 inputs (Item full name and count), one output (Item full name and a count
  up to that Item's `max-stack`), and a required `station`: `"hand"` or a
  Station tag. Names resolve after every Mod has declared. An unknown Item or
  tag is a load error naming the Recipe.
- `voxel-uses`, from any Mod, at most 64 per Mod. A row names a Material and a
  closed kind:
  - `door`, with an `open` Material. Both are solid Materials. The open one is
    the passable state the Game draws open: collision asks `Tables::blocks`,
    which lets bodies through it, while rays still stop on it. It gets no
    implicit Item of its own and cannot be placed.
  - `container`, with `slots` from 1 to 54 (27 when absent).
  - `station`, with a tag. The tag `hand` is reserved and a load error.
- A Material is at most one kind, and a Material is the closed or open state of
  at most one Door; either is a load error.
- `sound-cause` appends `door-open`, `door-close`, `container-open`,
  `container-close` and `craft`.
- Data-only Mods get the same rows as `[[recipes]]` and `[[voxel-uses]]` in
  their content file.

**Pure rules, in `tesserae-core`** (no Bevy, no wgpu)

- Prefactor: the slot rules (move, merge, swap, split, quick transfer, sort)
  work across any two slot sets, so the Inventory and a Container share them.
  The Inventory's own behaviour is unchanged.
- Craft rules: given the Inventory, a Recipe and a count, how many can be
  afforded (bounded by one output Stack), and spending and receiving as one step
  that returns what did not fit.
- The Recipe and voxel-use tables, resolved and validated, live beside the Item
  table.

**Server**

- Use no longer requires a Tool. The Server resolves Use in this order: a held
  Tool runs; otherwise a Usable Voxel within Edit reach is used; otherwise the
  selected Stack is eaten. A Usable Voxel past Edit reach but within twice it
  refuses Use as out of reach rather than eating. The aim answer's `use_aim`
  reports the Usable kind and its verb (Open, Close, Use Workbench, Open Chest),
  or the refusal (out of reach), for the prompt before the press.
- Door: Use toggles the aimed Voxel and the vertical run of connected Voxels of
  the same Door, at most 4, as one Edit group that never enters the undo
  journal. Shape and Orientation carry across. Closing into a Player's body is
  refused with Place's existing refusal ("You are standing there."). Removing
  the open Material drops the closed Material's Item. Undo and redo treat a
  Door's two Materials as the same Voxel.
- Station: Use opens a Station session for the Player at that Voxel. A Craft of
  a Station Recipe is refused unless the Player has a session open at a Station
  of that tag. A hand Recipe needs no session.
- Container: each Container Voxel has its Stacks, keyed by Voxel position. Use
  opens a session; the Server sends the Container's Stacks whole to every Player
  viewing it whenever they change. Inventory intents take a slot address that
  names an Inventory slot or a Container slot; Take all and Sort act on the
  Container. Moves are not Edits and never enter the journal.
- Sessions close when the Player leaves Edit reach of the Voxel, measured from
  the Eye to the Voxel's nearest point so a session opened at the edge of reach
  does not close at once, or the Voxel changes or is removed; the Server tells
  the Client, which closes the pane with a notice ("Out of reach of the
  Chest.").
- A Player's Remove of a Container that holds Stacks is refused ("Empty the
  Chest first."), and so is an undo or redo that would remove one. Anything else
  that removes or replaces it (a Mod Command, burning) spills its Stacks as
  Dropped items at the Voxel. Spilling is a per-tick check of every held
  Container rather than a hook in each removal path: a Voxel that is no longer a
  Container spills all its Stacks, and a smaller Container spills what no longer
  fits.
- Craft: a `Craft { recipe, count }` intent. Count is 1 or "as many as
  affordable". Refused with "Missing items." when nothing is affordable. Output
  that does not fit drops at the Player as Dropped items, reported like an undo
  refund that had no room. Not an Edit; never journalled. Same rules in both
  Game modes.
- Sound events: `door-open`/`door-close` at the Door, `container-open`/
  `container-close` at the Container, `craft` at the Player.

**Save**

- Container contents are an additive `containers` list in the World commit, read
  with a default of none, like dropped items: Voxel position, then Stacks by
  Item name. No format or generator version change.
- An entry whose Voxel is no longer a Container when the World opens spills as
  Dropped items there.
- Door state is the Voxel's Material, so it needs nothing new.

**Client** (as the mockup's step 16 section shows)

- The Inventory window's side pane gets tabs: Catalogue and Crafting in
  Creative, Crafting alone in Survival. Recipe rows show output, inputs and
  affordability; click selects, the footer shows have against need, Craft 1 and
  Craft N; double-click crafts one and Shift-click crafts N; a search field
  filters by output name.
- A Station opens the same pane titled by the Station, listing only its tag's
  Recipes. A Container opens its slot grid beside the Inventory with Take all
  and Sort; quick transfer crosses between the Container and the Inventory.
- The aim prompt sits under the reticle: `G` and the verb, or "Out of reach".
- The binding "Use tool" is renamed "Use"; G stays the default.
- The item details panel: icon beside the name in the chosen Shape; Count and
  Max in one row; Shape as five previewed chips; the actions in a two-by-two
  grid; no scrolling.
- The Client draws only what the Server sends: recipes from the Declaration,
  affordability from the Inventory it was sent, Container slots from the
  Container message.

**Terra**

- Items and Materials: Wood (exists), Workbench (Station, tag `workbench`),
  Chest (Container, 27 slots), Door (closed and open Materials).
- Recipes: Workbench from 4 Wood by hand; Chest from 8 Wood and Door from 3 Wood
  at the Workbench; a few more to reach 10 to 15 in all.
- CC0 sounds for the five new Causes, credited in its CREDITS file.

## Testing Decisions

A good test drives external behaviour: intents in, Server messages, save
contents or what is on screen out. It never asserts on private helpers, ECS
layout or how slots are stored.

Three seams, all existing:

1. **The Server headless App.** Prior art: the Inventory, Use refusal, eating,
   Drop and undo journal tests from steps 11 and 15. Covers: Craft spends and
   gives, Craft N bound by one Stack, overflow drop, "Missing items."; Station
   Recipes refused without a session and allowed with one at a matching tag; Use
   priority (Tool, Usable Voxel, eat); Door toggle of a 1 to 4 Voxel run, Shape
   and Orientation kept, closing on the body refused, no journal entry, undo
   matching either state, open Door dropping the closed Item; Container open,
   moves, quick transfer, Take all, Sort, two viewers seeing one contents;
   sessions closing on reach and on change; Remove, undo and redo of a non-empty
   Container refused; Mod Command replacement spilling.
2. **The save round trip through opening a World.** Prior art: the persistence
   rig's dropped-items and Inventory round trips. Covers: Container contents and
   an open Door survive reopen; a commit without `containers` opens with none;
   an orphaned entry spills.
3. **The Client App driven by key presses.** Prior art: the Inventory window,
   hotbar and history tests. Covers: the Crafting tab per Game mode, recipe
   selection and the craft buttons sending intents, the Station and Container
   panes following Server messages, the aim prompt, the renamed binding, and the
   compact item details.

Declaration parsing and validation (limits, unknown names, `hand` reserved, two
kinds on one Material, slot range) gets unit tests beside its code. The pure
slot and Craft rules get unit tests as the fast inner loop for seam 1.

Review-only screenshots of each pane (offscreen, one `/tmp` folder, never
committed, deleted after review) check the look against the mockup.

## Out of Scope

- Smelting, fuel, timed or progressive crafting, and anything that runs on its
  own (step 24).
- Shaped or grid Recipes; more than one output.
- Mod-drawn UI; a Mod supplies data only.
- Container hoppers, sorting rules, locks or ownership.
- Double doors, hinge-side choice, and redstone-style door triggers.
- Crafting from an open Container's Stacks.
- Recipes for the other demo Games.
- Network transport: sessions are per Player, but one Player still plays.

## Further Notes

- Ticket order: the slot prefactor, hand crafting, Doors and the item details
  start at once; Stations follow hand crafting and Doors; Containers follow the
  prefactor and Doors; sounds follow the features; docs come last.
- Risks: the Use intent changes shape (no Tool required), which touches the
  Client's Use routing and every Use test; Container sessions are the first
  per-Voxel Server state outside the Chunk; the removal paths that must spill
  are spread across edit, Mod Commands and combustion.
- The mockup is not copied into the engine.

## Amendment 2026-11-14: Doors in metres, Wood in Survival, new looks

Found while testing: a Door was a run of up to 4 Voxels, so at 0.25 m it stood 1
m tall and 0.25 m wide, shorter and narrower than the 1.8 m Player body; and
Terra had no Wood in Survival, so its Recipes were unreachable.

- A Door row declares its size in metres, width and height, defaulting to 1 m by
  2 m. One Door Item places the whole Door: ⌈size ÷ Voxel⌉ cells, one Voxel
  thick, across the Player's facing, as one undo group, refused if any cell is
  blocked. Use toggles the whole Door; breaking any cell removes it all and
  drops one Item. The 4-Voxel run limit is retired. (Ticket 09.)
- Terra scatters single Wood Voxels as a surface feature of its temperate Biome,
  every 6 m, so a Survival player can gather the Wood its Recipes need. (Ticket
  10.)
- The mockup gains a `/blocks` gallery of every Terra Material, exported from
  Terra's own textures, and the Workbench, Chest and Door get new looks there
  before they are committed. (Ticket 11.)
- A Door is a thin panel, not a block: placing forces a thin panel Shape across
  the Player's facing, the hinge on the aimed half, and Use turns each cell's
  panel 90° about its hinge. Collision follows the panel. (Ticket 13; a
  whole-panel swing is later work.) Fences get Gates, smaller Doors of 1 m by 1
  m. (Ticket 12.)
