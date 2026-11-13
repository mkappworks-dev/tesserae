# World and menu design prototype

Run `python3 server.py` from this directory, then open <http://127.0.0.1:8133>.
Use `--port` to choose another port. No dependencies or network services are
required. Use the server so direct page URLs work on reload.

Screens: title screen, Worlds, create World, edit World, Mods, Biomes, Review,
in-game menu, settings and an illustrative gameplay backdrop. Cinematic is the
only theme. It uses neutral charcoal surfaces, warm-white text and a muted sage
accent. The canvas landscape is illustrative; no external assets are required.

Drafts, created Worlds and preferences survive reload in this browser tab
through session storage. They are not real game saves. Clear this site's session
storage or open a fresh browser session to reset the samples.

The live scorecard covers 16 UI/UX areas. Read
[menu-research.md](menu-research.md) for sources, decisions, measured contrast
samples and the remaining 9.5+ criteria. Scores are provisional design
judgments, not player-test results.

New biome names, the expanded Terra set and other Games' names are design
proposals. Material pickers, package availability and dependencies are
illustrative. The three original Terra climate points, cave values and colours
are based on the current declarations. Existing Worlds only allow renaming;
generation changes create a new World. Review is named Review. Existing engine
trust requirements still apply separately when this flow is implemented.

This prototype does not modify the engine, access save files or grant trust to
any Game or Mod. Gamepad hints and actual engine rendering require later engine
implementation. Keyboard Tab, Enter and Escape work in the browser prototype.

Settings now shares one category-based screen between the title and in-game
menus: Behavior, Key bindings, Controller (including touch), Display, Audio and
Accessibility. The first four reflect current engine settings; Audio and
Accessibility remain proposed additions. Draft values survive category changes.
Apply, discard, reset and keyboard rebinding are interactive in this prototype.
Navigation containers fit their contents and scroll if needed.

Direct URLs include `/worlds`, `/worlds/new`, `/worlds/new/mods`,
`/worlds/new/biomes`, `/worlds/new/review`, `/worlds/1/edit`, `/mods`, `/menu`,
`/settings/behavior`, `/settings/key-bindings`, `/settings/controller`,
`/settings/display`, `/settings/audio` and `/settings/accessibility`. Browser
Back and Forward navigate between screens while retaining the draft. Reloading
restores the screen and the session draft.

Cinematic refinements include keyboard arrows, simulated controller D-pad/A/B
and shoulder handling, persistent field validation, modal focus isolation and
return, 100–200% text, reduced motion and a solid-background option. Actual
device, narration, localisation and engine performance tests remain necessary.

Menu containers use black surfaces over the brighter scenery. Sidebars use
consistent inline SVG icons and reserved icon/label/arrow columns. Settings
categories have fixed row geometry. Focus scrolls only its own content region.
Settings dropdowns use a custom listbox with keyboard arrows, Home/End,
typeahead, Enter, Escape, outside-click dismissal and simulated controller
navigation.

Catalogue now has a separate `/catalogue` screen with search, categories,
Material/Shape inspection and an Add to inventory action. `/inventory` is a
step-15 personal-storage preview with finite sample stacks. The proposed
gameplay keys are Tab → Menu, I → Inventory, C → Catalogue and F10 → Settings.
Escape backs out of overlays. Inside a menu, Tab still navigates focus. These
changes apply to the browser prototype only; engine defaults remain unchanged.
See [catalogue-inventory.md](catalogue-inventory.md) for current behavior, the
key-map proposal and the scope boundary with Inventory and Crafting.

Branding uses the current engine mosaic T, recreated as `brand.svg` from
`menu_ui.rs::make_logo`. Its five tiles use the theme's sage tones with a
charcoal grout line, and the wordmark uses the repository's Aileron SemiBold
font. The header and favicon share this mark. This replaces the mockup's generic
grid glyph; it is not a new logo design.

Inventory and Catalogue share one window with tabs. Inventory owns the hotbar;
Catalogue adds items to the backpack for arranging there. Inventory holds Items,
so a slot can differ from the Material it places: Terra's `rock` Item places
`terra:stone`, as declared for dropped items in step 12. The **Preview access**
control switches between Creative and Survival. Survival keeps the Catalogue as
a read-only reference and disables taking Materials, following Minetest Game's
creative check. It is a preview, not an engine permission system.

`/layout` is the quick layout overlay, opened with L during play. It mirrors the
engine's two layout kinds, Direction and count and Point to point, the 15-Voxel
gap limit and the 256-Voxel total. The preview shape is illustrative and edits
no terrain. Backspace leaves layout mode.

There is no separate Catalogue screen. In Creative, the Catalogue opens beside
Inventory: search or filter, then drag a Material onto any backpack or hotbar
slot, or pick a count (1–64) and send it to the backpack or the selected hotbar
slot. Double-click sends to the hotbar slot; Shift-click sends to the backpack.
A stack displaced from a slot moves to the backpack. C opens Inventory with the
Catalogue search focused, and `/catalogue` redirects there. Survival shows
Inventory alone.

Slots use a recessed bevel, a bright inner rim on hover and a lit frame for the
selected slot, in Inventory and in the in-game hotbar. The hotbar shows the held
item's name above it.

The Scores page also rates each screen. Shared fixes live in `polish.css`:
themed checkboxes, locked-field styling, range sliders, game-style frames, and
no menu header inside the game.

## Survival extension (proposed, awaiting approval)

Added after the menus were approved, for DESIGN.md steps 15 and 21. The
decisions it illustrates are in
[decisions.md](../.scratch/2026-11-05-phase-15-menus-and-inventory-ui/decisions.md).
Code is in `survival.js` and `survival.css`, and is throwaway.

- `/play` in Survival shows health and hunger as mosaic tiles over the hotbar,
  health on the left and hunger on the right, a half point split on the
  diagonal. Air appears above hunger underwater; its row is always reserved, so
  the HUD does not jump. A vignette marks low health. Creative shows no vitals.
- `/death` centres the cause, what it means, where your items are and for how
  long, then Respawn and Return to Worlds. Enter respawns; Escape does nothing
  there. The World behind greys and darkens, and the HUD hides.
- `/play` and `/death` show no footer: it belongs to the launcher and the menus.
  The hotbar sits 12 px above the window's edge.
- Create World has a Game mode picker; Edit World has the World's default mode
  and your own mode. The bar's **Your mode** control mirrors your mode in the
  open World.
- The dashed **Simulate** panel on `/play` is prototype chrome, not design. It
  places (also right-click), breaks, undoes (Z), picks up, fills every slot,
  falls, dives, drowns, touches lava, sprints, waits and eats (G), and toggles
  the Game's `keep_inventory`. It prints the full state after each action.
- Survival states: placing spends one Item; an empty slot refuses to place;
  undoing a placement refunds the Item, or drops it at your feet when there is
  no room; undoing a removal needs the Item and is refused without it; Pickup
  into a full Inventory takes what fits and leaves the rest on the ground, with
  a HUD notice; a full backpack reads "Full".
- Illustrative numbers: 20 health, 20 hunger, 10 air, falls over 3 m hurt 1 per
  metre, lava 4 per second, regeneration at hunger 18 or more, 5-minute despawn
  (the engine's step-12 value), regeneration 0.25 health per second (the Game
  declares it). `Melon` is Terra's food Item: a placeable solid worth 6 hunger,
  eaten with G, generated on grass.

In play, routine success shows no message: the hotbar count, the slots and the
vitals already change. Pickup shows a feed at the bottom right (same Item
merges, rows fade after 3 s). Refusals and the respawn message use the engine's
existing top-right notice card (`menu_ui::notify`); a refused Pickup uses the
notice above the hotbar. The bottom toast stays for menus only. The Simulate
panel moved to the top left and shows the last action.

Three HUD and three death-screen variants were compared; the tiles and the
centred screen were chosen, and the others removed.

Resolved: Survival hides the Catalogue, as the prototype does; the earlier
"read-only reference" wording is superseded.

## Engine parity additions

Added for ticket 23 so every engine screen has a mockup to compare against. They
reuse the existing components and tokens.

- Key bindings: every engine action, a primary and an alternative binding (empty
  reads "Add binding"), search, Movement / Building / Interface tabs, a reset
  per changed action, and the "Binding already in use" dialog.
- Controller: the controller button table and its help paragraph.
- Display: SMAA, a 1000 FPS cap, the sync notice, the frame-pacing summary and
  the footnote. Accessibility: 175% text.
- Create World: the resolved seed with Copy, and the cost of each Voxel size.
  Worlds: "No Worlds yet" when the list is empty.
- Edit World: Copy seed, locked Mods and Biomes, "Mode changes take effect when
  the World next opens.", Show folder and the folder line.
- `/worlds/new/trust` (the Trust sheet) and `/opening` (Opening a World).
- Inventory: the Shape picker in the inspector.
- Quick layout: the placement status line and the key-hint footer.
- Simulate (prototype chrome) gains Starve, Die with items despawned, and an
  "Engine states" group: Hold remove (break progress), Place hint, Measure tool,
  Hold Fire (a hand outside the hotbar) and the four save-status states.

## Crafting, Stations, Containers and Doors (step 16)

Approved, and implemented in DESIGN.md step 16. Illustrates that step. Code is
in `crafting.js` and `crafting.css`. The Wood, Glass, Brick, Workbench, Chest
and Door Items and every Recipe are illustrative Terra content, not
declarations.

Every Recipe names where it is crafted: `station = "hand"` or a Station's tag.
The field is required, and `hand` is reserved, so no Station may declare it.

|                  | Hand Recipe                                      | Station Recipe                                   |
| ---------------- | ------------------------------------------------ | ------------------------------------------------ |
| Example (mock)   | Workbench, Brick                                 | Chest, Door, Lamp                                |
| Declared as      | `station = "hand"`                               | `station = "workbench"`                          |
| Block needed     | None: craftable anywhere, any time               | A Workbench placed in the World                  |
| Opened by        | I, then the Crafting tab (`/inventory/crafting`) | Aim at the Workbench and press G (`/station`)    |
| Where you can be | Anywhere                                         | Within reach of it; walking away closes the pane |
| Lists            | Hand Recipes only                                | That Station's Recipes only                      |

Crafting works the same in both panes:

- Inputs come from the Inventory only, never from an open Container; output goes
  to the Inventory, and what does not fit drops at your feet with a notice.
- One craft makes at most one output Stack. It is instant: no time, fuel or
  progress. It is not an Edit, so undo (Z) does not reverse it.
- A Recipe you cannot afford is dimmed and its missing inputs are tinted. Click
  selects; the footer shows what you have against what it needs. Craft 1, or
  Craft N for as many as you can afford. Double-click crafts one; Shift-click
  crafts N.
- In Creative the Crafting tab sits beside the Catalogue; in Survival the side
  pane is Crafting alone.

- `/container`: Use on a Chest opens its 27 slots beside the Inventory.
  Click-to-move, drag and drop and Shift-click quick transfer cross between the
  Chest and the Inventory; Take all and Sort act on the Chest.
- `/play`: aimed at a Usable Voxel, a prompt under the reticle names what Use
  (G) would do: Open or Close a Door, Open Chest, Use Workbench. Out of reach,
  it says so before the press. Aimed at one, G uses it instead of eating.
- Refusals use the notice card: "Empty the Chest first." when breaking a Chest
  that holds Stacks, "You are standing there." when closing a Door on yourself,
  "Out of reach of the Chest." when walking away closes it.
- The **Use on Voxels** group in the Simulate panel aims at each kind, steps out
  of reach, stands in a doorway and breaks the Chest. The dashed "walk out of
  reach" button in a Station or Container pane is prototype chrome too.
