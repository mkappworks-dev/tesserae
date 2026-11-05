# Phase 13.1 decisions, before the spec

Taken with the maintainer on 2026-11-05, ahead of `/to-spec`. The spec cites
these; it does not reopen them.

## Scope

This branch builds everything the approved prototype shows, plus the screens
the prototype extension adds:

- Menus: title, Worlds, Create World (World, Mods, Biomes, Review), Edit World,
  the in-game Menu, and one shared Settings screen.
- HUD: the restyled hotbar with the held item's name, health and hunger.
- DESIGN.md step 15, Inventory, in full. It supersedes tickets 18 and 19 in
  `.scratch/2026-10-27-gameplay-performance-and-controls/`.
- DESIGN.md step 21, the Survival loop, in full: game mode, damage, hunger,
  death and respawn.
- Audio stays step 14. The Audio settings category is hidden until it lands.

## Biomes step

- Names are the declared Biome ids, title-cased (`ash_flats` → "Ash flats").
  The prototype's proposed names and the five-Biome Terra are dropped. No
  declaration or content change.
- Full editing, as approved: enable or disable, Climate position, cave scale,
  ground Materials from the selected Game and Mods, sky and fog colours, and the
  Climate sizes in metres. Reset to defaults. Locked after Create.
- A Create-time choice only changes the World's snapshotted Biome table
  (ADR-0022). The Server validates it at Create.

## Key bindings

- Tab → Menu, I → Inventory, C → Inventory with the Catalogue search focused,
  F10 → Settings (last category). Escape closes the current overlay, otherwise
  opens the Menu. E stays Flip, L stays Layout. Inside a menu, Tab moves focus.
- Help moves under Settings › Key bindings.
- A saved binding that still holds an old default (Tab → Catalogue, I → Help)
  is remapped once.

## Inventory (step 15)

- A slot holds an Item and a count. The Item places its declared Material;
  Shape and Orientation are the Player's placement choice (E flips, the
  Catalogue picks), so each Voxel costs one Item whatever its Shape.
- A Material with no declared Item gets an implicit Item of the same name.
- One Server-owned hotbar per Player per World replaces the per-Game Creative
  shortcuts. Existing shortcuts seed a new Creative World's hotbar.
- A fixed 27-slot backpack and 9-slot hotbar. `ItemDef` gains an optional
  `max_stack`, default 64, appended to the 0.3 contract.
- Pickup into a full Inventory leaves the Dropped item where it lies.
- On first open, a save's Carried totals pack into slots; overflow drops at the
  Player.
- The Inventory saves atomically with the World edits it pays for.
- Undo composes: undoing a placement refunds its Item in the same Server
  transaction; undoing a removal costs the Item and is refused if the Player
  does not hold it, despawning the removal's Dropped item if it still exists.
  A refund with no room drops at the Player's feet, as the Carried overflow
  does.

## Survival loop (step 21)

- Game mode: the World saves a default mode, Creative or Survival, chosen at
  Create and changeable in Edit World. New Worlds default to Creative. The
  Server stores each Player's mode, starting from that default. Creative has no
  damage or hunger, the Catalogue gives Items, and placing is free. Survival
  hides the Catalogue (C opens the Inventory with a Creative-only note),
  placing spends one Item, and Pickup is how slots fill.
- Content: the spec includes a ticket that gives Terra at least one food Item
  with its food value, so hunger can be tested in a real Game.
- Damage, computed by the Server in metres from Game-declared numbers: fall
  damage above a safe height, drowning after a breath time underwater, and
  contact damage from a Material declared harmful, per second. Mobs are step
  22.
- Hunger: a meter drains over time and faster while sprinting. Health
  regenerates above a threshold; at zero the Player starves. `ItemDef` gains an
  optional food value; eating is a Use intent (step 11). The Game declares the
  rates.
- Death drops the whole Inventory as Dropped items at the death point (step
  12's despawn), unless the Game declares `keep_inventory`. A death screen
  offers Respawn or Return to Worlds. Respawn is at the World spawn with full
  health and hunger. Beds and set spawns come later.

## Prototype verdict

- Survival HUD: mosaic tiles over the hotbar. Health left, hunger
  right, air above hunger. The air row's height is always reserved so the HUD
  does not jump on diving. Toasts stack above the whole HUD column.
- The "Inventory full" notice appears only after a Pickup leaves something on
  the ground, and clears when a slot frees.
- In-play messages: no message for routine success (place, break, eat,
  damage). Pickup shows a bottom-right feed that merges the same Item and fades
  after 3 s. Refusals and the respawn message use the existing top-right notice
  card (`menu_ui::notify`), with repeats silent as in step 11. The bottom toast
  stays for menus.
- Death screen: centred: the cause, what it means, where the Items
  are and for how long, then Respawn (Enter) and Return to Worlds.

## Process

- Extend the prototype first with the screens it lacks: the health and hunger
  HUD, the death screen, the mode picker in Create World and Edit World, and
  Survival Inventory states (counts spent by placing, a full backpack refusing
  Pickup, an undo refused for a missing Item). The maintainer approves them
  before the spec.
- The maintainer updates the `mattpocock-skills` plugin; the spec, tickets,
  triage and implementation use its `/to-spec`, `/to-tickets`, `/triage` and
  `/implement`.
- Rebase onto origin/main once the phase 13 PR merges. Do not commit or push
  until asked.

## During implementation

Taken with the maintainer on 2026-11-05, while the tickets were being built.
The prototype shows the ones with a screen.

- Terra generates melons on grass. `terra:melon` is a placeable solid whose
  Item is food worth 6; G eats it. Every Terra World's terrain changes.
- The Game declares the health regeneration rate in its Survival rules,
  default 0.25 health per second.
- The death screen counts down to the moment the dropped Items despawn, and
  the respawn notice gives the same time left. The Server owns the deadline.
- An undo entry pays and refunds by the mode it was made in, so a switch
  between Creative and Survival cannot mint or destroy Items.
- Anything that does not fit in the Inventory drops at the Player, including
  an undeclared Item's Carried overflow.
- The Inventory dims the World behind it without blur, as Minecraft (blur on
  menus only, by design) and Luanti (a translucent fullscreen colour) do. The
  Menu, Settings and Create World keep their blur.
- Any solid Material, food included, can be a Biome's ground.
- At the end the branch, this folder and these documents are renamed from
  phase 13.1 to phase 15.
- Every demo Game declares its own food as a surface feature of its Biomes, not
  only Terra: the terra melon, meadow pumpkin, ember pepper, tidewater coconut
  and lumen glowberry. Variety (crops, cooking, meat) waits for steps 16, 18 and
  22.
- A Biome declares surface features in the 0.3 contract: Materials the Engine
  stands one Voxel above the Biome's surface at a mean spacing in metres, pinned
  with the World's Biome table and absent at coarse levels. Mods still place no
  terrain.
