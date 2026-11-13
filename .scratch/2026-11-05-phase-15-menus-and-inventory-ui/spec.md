Status: done

# Phase 15: Menus, Inventory and the Survival loop

The approved menu and HUD prototype, DESIGN.md roadmap step 15 (Inventory) and
step 21 (Survival loop), built together on one branch. Stacked on phase 13
(Biomes); a Create-time Biome choice only changes the snapshotted Biome table
(ADR-0022).

Source material beside this spec: [decisions.md](decisions.md) records every
decision taken before it, and [mockup/](../../mockup/) is the approved browser
mockup (`python3 server.py` there; its README lists every screen). Vocabulary
is in [GLOSSARY.md](../../GLOSSARY.md); new terms are listed under
Implementation Decisions and land in the glossary with the docs ticket.

This spec supersedes tickets 18 and 19 of
`.scratch/2026-10-27-gameplay-performance-and-controls/`, which deferred owned
inventory until its policies were decided. They are decided in decisions.md.

## Problem Statement

A player opens Tesserae to a World list and a Create form that asks for a name,
a Game, Mods, a seed and a Voxel size. They cannot see or shape the Biomes a
World will have. In a World, Tab opens a Catalogue that is also the settings
menu, I opens Help, and nothing is the game menu players expect from Escape or
Tab.

Every Material is free and unlimited. Items picked up after step 12 become a
line of text, "carried: Rock x48", that cannot be placed, moved or used. The
nine quick slots are Client-side shortcuts saved per Game outside any World,
so they say nothing about what the Player has. There is no reason to gather
anything: no health, no hunger, no danger, no death. A Game author cannot make
a World that is about surviving, and a player cannot choose between building
freely and playing for keeps.

## Solution

The menus become the approved Cinematic set: a title screen, Worlds, a
four-step Create World (World, Mods, Biomes, Review), Edit World, an in-game
Menu, and one Settings screen shared by the title and the game. The Biomes step
shows the Biomes the chosen Game and Mods declare and lets the player turn
them on or off and tune them before the World exists.

Each Player has a Server-owned Inventory: a 27-slot backpack and a 9-slot
hotbar, holding stacks of Items. Each World has a default game mode and each
Player has their own. In Creative, the Catalogue opens beside the Inventory and
gives Items, and placing is free. In Survival, the Catalogue is closed, placing
a Voxel spends one Item, Pickup is how slots fill, and undo pays and refunds
Items in the same Server transaction.

Survival also brings health, hunger and air. Falling too far, running out of
air underwater and touching a harmful Material hurt; hunger drains over time,
faster when sprinting, and food refills it. At zero health the Player dies,
drops their Inventory where they fell, and respawns at the World spawn.

The HUD shows health and hunger as mosaic tiles over the hotbar, air above
hunger underwater, a pickup feed at the bottom right, and refusals in the
engine's existing top-right notice card. The key map becomes Tab → Menu,
I → Inventory, C → Catalogue, F10 → Settings.

## User Stories

Menus and Worlds

1. As a player, I want a title screen with Continue, Your Worlds, Create a
   World, Mods and Settings, so that I start where I left off or begin
   something new in one step.
2. As a player, I want Continue to name the World and Game I last played, so
   that I know what it will open.
3. As a player, I want my Worlds listed with their Game, Voxel size and when I
   last played, so that I can find the one I want.
4. As a player, I want to search my Worlds by name or Game, so that a long
   list stays usable.
5. As a player, I want Play and Edit side by side on each World, so that both
   are one click away.
6. As a player, I want the Mods library reachable from the title screen, so
   that I can see what is installed before I create a World.
7. As a player, I want the brand mark and wordmark to match the engine's
   mosaic logo, so that the menus feel like one product.
8. As a keyboard player, I want Tab, Enter, Escape and the arrow keys to work
   on every menu screen, so that I never need the mouse.
9. As a controller player, I want the D-pad, A, B and the shoulders to move
   through menus and categories, so that I can play from the couch.

Create World

10. As a player, I want Create World as four steps (World, Mods, Biomes,
    Review) with a rail showing where I am, so that a long setup stays calm.
11. As a player, I want to name the World, choose a Game and plant a seed on
    the first step, so that the essentials come first.
12. As a player, I want to choose Creative or Survival on the first step, so
    that the World starts the way I mean to play it.
13. As a player, I want the Voxel size under More World options, so that the
    default stays simple.
14. As a player, I want the Mods step to show which Mods the Game requires and
    keep them on, so that I cannot make a World that will not load.
15. As a player, I want the Biomes step to list the Biomes my Game and Mods
    declare, named from their declared names, so that I know what the World
    can contain.
16. As a player, I want to turn a Biome off, so that a World has no desert if I
    do not want one.
17. As a player, I want to move a Biome's temperature and humidity on a Climate
    map, so that I can make one region more or less common.
18. As a player, I want to scale or switch off a Biome's caves, so that a plain
    can stay walkable.
19. As a player, I want to choose a Biome's ground Materials from the selected
    Game and Mods, so that I can make a snow plain or a sand coast.
20. As a player, I want to change a Biome's sky and fog colours, so that the air
    matches my idea of the place.
21. As a player, I want to set the Climate sizes in metres, so that regions are
    as broad as I want at any Voxel size.
22. As a player, I want Reset to defaults on the Biomes step, so that I can
    undo my tuning in one action.
23. As a player, I want a Review step that summarises the World, Mods, Biomes
    and game mode with Edit links, so that I can check everything before
    creating.
24. As a player, I want invalid values (an empty Biome set, a Climate size out
    of range, a name over 64 characters) held on their step with a message, so
    that I fix them where they are.
25. As a player, I want leaving an unfinished setup to ask before discarding,
    so that I do not lose it by accident.
26. As a player, I want my Biome choices to apply only to the new World, so
    that other Worlds and the Game are untouched.

Edit World

27. As a player, I want to rename a World without changing its terrain, so that
    I can tidy my list.
28. As a player, I want to change a World's default game mode in Edit World, so
    that new Players join the way I now intend.
29. As a player, I want to change my own game mode in Edit World, so that I can
    switch between building freely and playing for keeps.
30. As a player, I want the Game, seed, Voxel size and Biomes shown as locked,
    so that I understand generation is fixed at Create.
31. As a player, I want Create from these settings, so that I can start a
    variant of a World I like.
32. As a player, I want to delete a World behind a confirmation, so that I do
    not lose one by a stray click.

In-game Menu and Settings

33. As a player, I want Tab or Escape to open a Menu with Resume, Inventory,
    Settings, World details and Return to Worlds, so that the game menu is
    where I expect it.
34. As a player, I want Escape to close an open overlay before it opens the
    Menu, so that backing out is predictable.
35. As a player, I want F10 to open Settings at the category I used last, so
    that I can change one thing quickly.
36. As a player, I want one Settings screen with Behavior, Key bindings,
    Controller, Display and Accessibility, from the title and from the game,
    so that every setting lives in one place.
37. As a player, I want setting changes to wait for Apply, with Discard and
    Reset, so that I can try values safely.
38. As a player, I want to rebind every key, so that the controls suit me.
39. As a returning player, I want my old bindings for the Catalogue (Tab) and
    Help (I) moved to the new defaults once, so that I am not stuck with keys
    that now mean something else.
40. As a player, I want bindings I chose myself kept through that change, so
    that my customisation survives.
41. As a player who needs larger text, I want text from 100% to 200%, so that I
    can read every menu.
42. As a player sensitive to motion, I want a reduced motion setting that also
    stills the HUD's pulses and slides, so that the game is comfortable.
43. As a player, I want a solid background option, so that menus never sit on
    busy scenery.
44. As a player, I want World details in the Menu to show the Game, Voxel size,
    seed, Biomes and both game modes, so that I can check them in play.

Inventory (step 15)

45. As a player, I want I to open my Inventory over the dimmed World, so that I
    can arrange what I carry without leaving it.
46. As a player, I want a 27-slot backpack and a 9-slot hotbar, so that what I
    carry is finite and familiar.
47. As a player, I want the hotbar in my Inventory to be the same hotbar I use
    in play, so that arranging it is direct.
48. As a player, I want to click a stack then a slot to move, merge or swap,
    and to drag and drop as well, so that both habits work.
49. As a player, I want Shift-click to move a stack between backpack and
    hotbar, so that quick access is one action.
50. As a player, I want to split a stack in half, so that I can share it
    between slots.
51. As a player, I want Sort to merge and order my backpack, so that it stays
    tidy.
52. As a player, I want item details (name, source, the Material it places,
    count, stack limit), so that I know what each stack is.
53. As a player, I want each Item to stack up to the limit its Game declares, so
    that a tool and a building Material stack differently.
54. As a player, I want the backpack to read "Full" when every slot is used, so
    that I know before Pickup fails.
55. As a player, I want digits 1-9 and the scroll wheel to select a hotbar slot,
    so that changing what I hold is instant.
56. As a player, I want the held Item's name above the hotbar, so that I know
    what I will place.
57. As a player, I want the Shape I choose (E flips it, the Catalogue picks it)
    to apply to whatever Material my held Item places, so that one stack of
    Rock builds cubes, slopes and slabs.
58. As a returning player, I want the Items I had Carried before this update
    packed into my slots, with any overflow dropped beside me, so that nothing
    I gathered is lost.

Creative

59. As a Creative player, I want the Catalogue beside my Inventory with search
    and categories, so that I can find any Material quickly.
60. As a Creative player, I want to drag a Material onto any slot, so that I
    fill my hotbar the way I like.
61. As a Creative player, I want to pick a count from 1 to the stack limit and
    send it to the backpack or the selected hotbar slot, so that I get exactly
    what I need.
62. As a Creative player, I want double-click to send to the hotbar slot and
    Shift-click to send to the backpack, so that common moves are fast.
63. As a Creative player, I want C to open the Inventory with the Catalogue
    search focused, so that finding a Material is one key.
64. As a Creative player, I want placing to be free and to take no damage or
    hunger, so that I can build without interruption.
65. As a Creative player, I want removing a Voxel to drop nothing, so that
    building does not litter the World.
66. As a Creative player in a new World, I want my old per-Game quick slots to
    seed the hotbar, so that my familiar set is ready.

Survival

67. As a Survival player, I want placing to spend one Item from the held stack,
    so that building costs what I gathered.
68. As a Survival player, I want placing with an empty slot or a non-placeable
    Item refused with a notice, so that I know why nothing happened.
69. As a Survival player, I want a Layout refused before it changes anything
    when I do not hold enough Items for every Voxel, so that I never get half a
    wall.
70. As a Survival player, I want removing a Voxel to drop its Item for Pickup,
    so that I can gather the World.
71. As a Survival player, I want Pickup to fill matching stacks first, then
    empty slots, hotbar before backpack, so that Items land where I use them.
72. As a Survival player, I want Pickup into a full Inventory to take what fits
    and leave the rest on the ground, with a notice above the hotbar naming
    what was left, so that I know to make room.
73. As a Survival player, I want each pickup shown briefly at the bottom right,
    merging repeats of the same Item, so that I see what I gained without it
    covering the World.
74. As a Survival player, I want undoing a placement to refund its Item, so that
    a mistake costs nothing.
75. As a Survival player, I want a refund with no room to drop at my feet, so
    that undo never fails because my backpack is full.
76. As a Survival player, I want undoing a removal to spend the Item and be
    refused if I do not hold it, so that undo cannot create Items.
77. As a Survival player, I want undoing a removal to remove its Dropped item
    if it is still lying there, so that undo cannot duplicate Items.
78. As a Survival player, I want the Catalogue closed, and C to tell me it is
    Creative-only, so that Survival stays about gathering.
79. As a Survival player, I want flight off, so that the World's distances and
    dangers are real.

Vitals, damage, hunger, death

80. As a Survival player, I want health and hunger as mosaic tiles above the
    hotbar, half points split on the diagonal, so that I read them at a glance.
81. As a Survival player, I want air shown above hunger only underwater, with
    its space always reserved, so that the HUD does not jump when I dive.
82. As a Survival player, I want a red edge on the screen at low health and a
    brief flash when hurt, so that danger is felt, not only read.
83. As a Survival player, I want falls over the Game's safe height to hurt in
    proportion to the height in metres, so that a long fall is dangerous at
    any Voxel size.
84. As a Survival player, I want air to drain while my Eye is under a fluid and
    drowning damage at zero, so that deep water is a risk.
85. As a Survival player, I want Materials the Game declares harmful, like lava,
    to hurt per second while I touch them, so that hazards matter.
86. As a Survival player, I want hunger to drain over time and faster while
    sprinting, so that food matters.
87. As a Survival player, I want health to regenerate while hunger is high, so
    that resting heals me.
88. As a Survival player, I want starvation damage at zero hunger, so that
    hunger has a consequence.
89. As a Survival player, I want to eat the food Item in my selected slot with
    Use (G), refused when I am full, so that eating is deliberate.
90. As a Survival player, I want death to tell me the cause, where I fell and
    how long my Items stay there, so that I can decide whether to go back.
91. As a Survival player, I want my Inventory dropped where I died unless the
    Game keeps it, so that death has a cost I can recover.
92. As a Survival player, I want Respawn (Enter) at the World spawn with full
    health, hunger and air, and a notice saying how far away my Items are, so
    that I can go and get them.
93. As a Survival player, I want Return to Worlds on the death screen, so that I
    can stop after a death.
94. As a Survival player, I want a death to survive quitting, so that reopening
    the World shows the death screen, not a free revival.

Game and Mod authors

95. As a Game author, I want to declare an Item's stack limit, so that a sword
    stacks to 1 and sand to 64.
96. As a Game author, I want to declare an Item as food with a hunger value, so
    that my Game has things to eat.
97. As a Game author, I want to declare a Material's contact damage per second,
    so that lava and thorns hurt.
98. As a Game author, I want to declare the Survival numbers (maximum health,
    hunger and air, safe fall height, damage rates, hunger drain and regen
    threshold) and keep_inventory, so that my Game sets its own difficulty.
99. As a Game author who declares none of these, I want sensible engine
    defaults, so that an existing Game is playable in Survival unchanged.
100. As a Mod author, I want to add food Items and harmful Materials to any
     compatible Game, so that a data Mod can enrich Survival.
101. As a Mod author, I want the Survival numbers to stay the Game's, so that a
     Mod cannot silently change a Game's difficulty.
102. As a Terra player, I want at least one food Item in Terra, so that hunger
     can be survived in the reference Game.

Engine maintainers

103. As an engine maintainer, I want every Inventory change made by the Server
     in answer to a Client intent, so that the Client never mutates World or
     Player state (ADR-0002).
104. As an engine maintainer, I want the Inventory rules (fit, merge, spend,
     split, sort) as a pure module in `tesserae-core`, so that they are fast to
     test and free of Bevy and wgpu.
105. As an engine maintainer, I want a placement and its Item spend in one undo
     entry, so that undo and redo stay all-or-nothing.
106. As an engine maintainer, I want damage, hunger and air computed in metres
     and seconds on the Server tick, so that they hold at every Voxel size and
     frame rate.
107. As an engine maintainer, I want new declaration fields appended to the 0.3
     contract and documented under "Amended before release", so that the
     contract history stays honest.
108. As an engine maintainer, I want new save data as additive Player
     extensions and an optional World field, so that saves from phase 13 keep
     opening.
109. As an engine maintainer, I want the HUD drawn only from Server messages, so
     that what the Player sees is what the Server holds.
110. As a contributor, I want the glossary, ADRs, the contract compatibility doc
     and DESIGN.md's step status updated, so that the next reader finds the
     decisions.

## Implementation Decisions

**Vocabulary** (glossary entries land with the docs ticket)

- **Inventory**: the Server-owned stacks a Player holds: a 27-slot backpack and
  a 9-slot hotbar. Replaces Carried, which is retired.
- **Stack**: one Item and a count, at most the Item's stack limit.
- **Hotbar**: the nine Inventory slots usable in play; one is selected.
- **Catalogue**: the Creative-only list of Materials a Player can take Items
  from. Not the Inventory and not ownership.
- **Game mode**: Creative or Survival. A World has a default; each Player has
  their own.
- **Vitals**: a Player's health, hunger and air.
- Shape and Orientation stay the Player's placement choice, not part of a
  Stack.

**Inventory, in `tesserae-core`** (no Bevy, no wgpu)

- A pure Inventory module owns the slot array and every rule: receive (merge
  into matching stacks, then empty slots, hotbar before backpack; return what
  did not fit), spend from the selected slot or by Item, move, merge and swap
  between slots, split, sort, quick transfer between backpack and hotbar, and
  "can this whole Layout be paid".
- Stack limits come from the Item table. Counts never exceed them and never
  reach zero in a slot; an emptied slot is empty.
- A Material with no declared Item gets an implicit Item of the same name, so
  every placeable Material, including fluid sources, has an Item. Ignite stays
  a Tool (step 11), not an Item.

**Server**

- The Inventory lives on the Player, beside the Vitals and the Player's game
  mode. The Server still has one Player; the data is shaped per Player so that
  multiplayer needs no format change.
- New Client intents, each validated by the Server: select a hotbar slot; move,
  split, quick-transfer and sort; take from the Catalogue (Item, count, target
  slot or backpack), refused unless the Player is in Creative; eat (through the
  existing Use verb with a food Item selected).
- The Server answers with the full Inventory (36 slots is small) whenever it
  changes, and with a refusal reason otherwise. Refusals join the existing
  reject reasons with player-facing messages: nothing held, the held Item
  places nothing, not enough Items for this Layout, the Catalogue is
  Creative-only, not hungry, no room.
- Place in Survival resolves the Item in the Player's selected hotbar slot on
  the Server, checks that it places the intent's Material, and spends one. The
  intent's Shape and Orientation still apply. In Creative nothing is spent.
- A Layout in Survival is paid up front: if the Inventory cannot pay for every
  Voxel the Layout is refused before any Voxel changes.
- Removal in Survival drops the Material's Item as today (step 12). In Creative
  a removal drops nothing.
- Pickup feeds the Inventory instead of Carried. What does not fit stays as a
  Dropped item with its count reduced; the Server tells the Client what was
  left so the HUD can name it.
- Dropping a death Inventory or an undo refund needs a spawn that takes a
  position in metres and spreads several stacks; the current spawn takes a
  Voxel coordinate.

**Undo composes with the Inventory** (DESIGN.md step 15)

- An undo entry records the Item it paid or the Dropped item it produced, and
  whether it was free (Creative). Restore checks the Inventory precondition
  with the Voxel preconditions and applies all of it or none of it.
- Undoing a paid placement refunds the Item; with no room the refund drops at
  the Player's feet. Undoing a removal spends the Item and is refused without
  it; if the removal's Dropped item still exists it is despawned instead of
  spending. Redo is the mirror image.
- Entries made in Creative are free in both directions, even after the Player
  switches to Survival. Death clears the Player's undo history.

**Game mode**

- The World's default mode is World state: an optional field in the save,
  absent meaning Creative, mirrored in the World's sidecar for the World list.
  Each Player's mode is a Player save extension that starts from the default.
- New Worlds default to Creative. Create World writes the chosen default.
- Edit World writes requested mode changes to the sidecar as pending; the
  Server applies and clears them when the World next opens. No Client changes
  a mode during play.
- Survival disables flight; the Server refuses the switch.

**Vitals, damage and hunger** (Server, 64 Hz tick, metres and seconds)

- Damage applies in Survival only. Fall damage comes from the landing impact
  speed, converted to a fall height in metres (v² / 2g), above the Game's safe
  height. Air drains while the Eye is below a fluid surface (the Server already
  tracks immersion) and refills on surfacing; at zero the Player takes
  drowning damage per second. Contact damage applies per second while the
  Player's body overlaps a Material that declares it.
- Hunger drains per minute, faster while sprinting. Health regenerates while
  hunger is at or above the threshold. At zero hunger the Player starves.
- The Server sends the Vitals to the Client when they change, with the cause
  of the last damage for the HUD flash.
- Engine defaults when a Game declares nothing: 20 health, 20 hunger, 10 s of
  air, safe fall 3 m then 1 per metre, drowning 2 per second, hunger 1 per
  minute idle and 2 sprinting, regeneration at hunger 18 or more, starvation 1
  per 4 s, keep_inventory off. These are rules, not content (ADR-0016 is about
  Materials).

**Death and respawn**

- At zero health the Server records a death (cause, position in metres,
  dropped stacks), drops the Inventory as Dropped items unless the Game
  declares keep_inventory, clears the undo history, and ignores movement and
  edit intents until respawn. The death is saved, so reopening shows it.
- Respawn reuses the existing respawn intent: the World spawn, full Vitals, and
  a notice with the distance to the death point and the 5-minute despawn.

**Declarations and contract** (0.3, appended, "Amended before release")

- Item: optional stack limit (default 64) and optional food value.
- Material: optional contact damage per second.
- A new optional Survival record, honoured from the Game only: maximum health,
  hunger and air, safe fall height, fall damage per metre, drowning damage,
  hunger drain idle and sprinting, regeneration threshold, starvation damage,
  keep_inventory. A Mod's Survival record is refused.
- Every field crosses WIT; the Rust SDK, the data-only form, the starter and
  the contract compatibility doc gain them. No per-voxel host call.
- Terra gains at least one food Item, and lava (where a Game declares it)
  gains contact damage, in a content ticket.

**Save**

- New Player extensions for the Inventory, the Vitals and death, and the
  Player's mode; one optional World field for the default mode. The extension
  allow-list must name each new key, or older builds refuse the save, which is
  the intended forward-compatibility behaviour.
- The legacy Carried extension is read once: its totals pack into slots and
  any overflow drops at the Player. It is not written again.
- Inventory and Vitals save in the same commit as the World edits they paid
  for, joining the existing dirty and quit checks.
- The per-Game Creative quick-slot file is retired. On the first open of a new
  Creative World the Client offers its stored slots as Catalogue takes, which
  the Server validates like any other; the file is not written again.

**Create World and the Biomes step**

- The Biomes step needs the declarations of the selected Game and Mods before
  the World exists; the launcher reuses the Mod host's declaration pass for a
  preview, under the same trust rules as opening.
- The player's Biome edits (enabled set, Climate point, cave scale, ground
  Materials, sky and fog colours, Climate sizes in metres) go into the sidecar
  with the pending World. On first open the Server applies them to the Biome
  table before it is snapshotted, validating ranges, Material names and at
  least one enabled Biome. The Game's declarations are untouched (ADR-0022).
- Biome names are the declared names, title-cased ("ash_flats" → "Ash flats").
  No Game content changes.

**Client: menus, Settings and key bindings**

- The World list becomes the title screen, Worlds, Create World with its four
  steps and Trust where needed, Edit World and the Mods library, in the
  prototype's layout and copy.
- The in-game Menu (Resume, Inventory, Settings, World details, Return to
  Worlds) replaces the Catalogue-plus-settings menu. Return to Worlds uses the
  existing save-and-quit.
- Settings becomes one screen with Behavior, Key bindings, Controller
  (including touch), Display and Accessibility, shared by the title and the
  game; drafts apply on Apply as today. The existing Movement, Building and
  Interface settings map into these. Audio is hidden until step 14.
- New default keys: Tab → Menu, I → Inventory, C → Inventory with the Catalogue
  search focused, F10 → Settings. Escape closes the current overlay, otherwise
  opens the Menu. E stays Flip, L stays Layout, G stays Use. Inside a menu, Tab
  moves focus. Help leaves I; what it showed moves under Settings.
- The profile's version moves to 2 with a one-time migration: a binding still
  equal to an old default (Tab → Catalogue, I → Help) takes the new default;
  any other binding is kept.

**Client: HUD and overlays** (the prototype's `hud.css`, `polish.css` and
`survival.css` are the look)

- Hotbar: recessed bevel slots, a bright rim on hover, a lit frame on the
  selected slot, the held Item's name above.
- Vitals (Survival only): health tiles on the left and hunger tiles on the
  right above the hotbar, half points split on the diagonal, the air row above
  hunger with its height always reserved, a low-health vignette and a hit
  flash, both stilled by reduced motion.
- Pickup feed at the bottom right: newest on top, the same Item merges and
  moves to the top, rows fade after 3 s. It replaces the "carried:" line and
  the "+N" toast.
- Refusals and the respawn message use the existing top-right notice card;
  repeats while a button is held stay silent, as in step 11. A refused Pickup
  uses the persistent notice above the hotbar instead. The bottom toast stays
  for menus.
- Inventory overlay: centred window over the dimmed, blurred World; in
  Creative the Catalogue sits beside it. Survival shows the Inventory alone.
- Death screen: centred over the darkened World: the World and distance from
  spawn, the cause, what it means, where the Items lie and for how long, then
  Respawn (Enter) and Return to Worlds. Escape does nothing there.
- The Client draws Vitals, the Inventory and death only from Server messages.

**ADRs** (next free numbers at merge)

- "The Inventory is Server-owned and pays inside the undo journal": why undo
  composes instead of being disabled in Survival, and why a refund with no
  room drops instead of failing.
- "Game mode is per Player with a World default": why not a World-only
  setting, and why mode changes apply at open.

## Testing Decisions

A good test drives external behaviour: intents in, Server messages, save
contents or what is on screen out. It never asserts on private helpers, ECS
layout or how slots are stored in memory.

Three seams, all existing:

1. **The Server headless App.** Intents in, Server messages out. Prior art: the
   helpers in the Server edit tests, the Pickup and Carried tests in the Server
   entities tests, the Use refusal tests and the undo journal bound tests.
   Covers: Inventory moves, splits, sorts and transfers; Catalogue takes
   allowed in Creative and refused in Survival; placing spends in Survival and
   not in Creative; an empty or non-placeable slot refused; a Layout refused up
   front when unaffordable; Pickup merge order and partial Pickup; undo
   refunds, refunds that drop, removal undo refused or paid by despawning its
   drop; Creative entries free after a mode switch; fall, drowning and contact
   damage in metres at two Voxel sizes; hunger drain, sprint drain,
   regeneration and starvation; eating refused when full; death drops,
   keep_inventory, ignored intents while dead, and respawn; flight refused in
   Survival.
2. **The save round trip through opening a World.** Prior art: the persistence
   test rig on Terra, its dropped-items-and-Carried and movement round trips,
   and the core save recovery tests. Covers: Inventory, Vitals, death and both
   modes survive reopen; a phase 13 save with Carried opens, packs into slots
   and drops overflow; a pending mode change applies at open; the Biome edits
   from Create are the snapshotted table and the Game's table is unchanged.
3. **The Client App driven by key presses.** Prior art: the Client edit tests,
   the Layout menu tests, the history tests and the World list tests. Covers:
   the new key map and its one-time migration; Escape closing overlays before
   the Menu; the Create World steps, validation and Review; Edit World writing
   pending mode changes; the HUD, pickup feed, notices and death screen
   following Server messages; Survival closing the Catalogue.

Declaration parsing (stack limit, food, contact damage, the Survival record, a
Mod's Survival record refused) gets small unit tests beside its code. The pure
Inventory module gets its own unit tests for the fit and merge rules, as the
fast inner loop for seam 1.

Review-only screenshots of each screen (offscreen, one `/tmp` folder, never
committed, deleted after review) check the look against the prototype.

## Out of Scope

- Audio and the Audio settings category (step 14).
- Crafting and stations (step 16); armour, equipment and durability.
- Throwing or discarding an Item from the Inventory.
- Chests or any container other than the Player's Inventory.
- Mobs and the damage they deal (step 22); beds and set spawn points.
- Multiplayer: the Server still holds one Player, though the data is shaped
  per Player.
- Commands and permissions to change modes in play (step 26).
- New Biome content or renamed Biomes; the prototype's proposed names and the
  five-Biome Terra are dropped.
- Localisation, narration and physical controller testing beyond today's
  support.
- `create-world-variants.html`, which stays reference material only.

## Further Notes

- Size: three roadmap pieces on one branch. Tickets should be thin vertical
  slices that each leave the game playable, starting with the pure Inventory
  module and the Server Inventory in Creative, then Survival placement and
  Pickup, then undo, then Vitals, then death, with menus and the Biomes step
  in parallel where they do not touch the same code.
- Risks: the launcher needs Game and Mod declarations before a World opens;
  the Server's single-Player assumption is everywhere; the save extension
  allow-list must grow with every new key; the profile has had no migration
  before.
- The prototype is throwaway and is not copied into the engine. Its losing
  variants were removed; only the chosen designs remain.
- Rebase onto origin/main once the phase 13 PR merges.
