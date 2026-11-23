# 01: Update the mockup

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

Update `mockup/` for the spec's mockup changes: the trust modal's Details toggle (permissions and limits in a scrolling list, Cancel focused), the Edit World discard dialog (Keep editing focused), the Biomes side panel with Temperature and Humidity sliders and Caves, the Inventory search as a field under the tabs, the Layout-ready card with L Edit, ⌫ Exit and RMB Place as key caps, and a new Place Tool page whose side panel works like Quick layout's, using the overlay.

## Acceptance criteria

- [x] Each change is reachable at a mockup route or state, listed in this ticket's Comments
- [x] The Place Tool page has its own route
- [x] `npm run fmt:check` passes

## Comments

Where to see each change (`python3 mockup/server.py`, then these routes;
states set from the browser console where there is no route):

- Trust modal Details: `/worlds/new/mods`, then `state.trusted = false;
  goStep(3)`. Cancel is focused; Details (closed by default) lists each code
  item's permissions and the host limits from `tesserae_mods::trust_prompt`,
  and names the data-only items. The item list and the Details list both
  scroll inside the modal, which stays inside the window. The full Trust page
  (`/worlds/new/trust`) is unchanged.
- Edit World discard dialog: `/worlds/1/edit`, change the name or mode, then
  Esc (or the Esc Cancel prompt). Keep editing is focused; Discard returns to
  Worlds unsaved. Without edits Esc leaves at once.
- Biomes panel: `/worlds/new/biomes`, then click a biome's pencil. The panel
  heads with the biome's diorama; Temperature, Humidity and Caves are one-line
  sliders above the climate map, then Ground and Atmosphere (Sky and Fog on one
  row). Also fixed: editing now flips the panel's status to Customised (a
  filter checkbox shared its `data-biome-status` selector).
- Inventory search: `/inventory` and `/inventory/crafting`. A search field
  under the tabs, always shown; / focuses it and the prompt bar shows / Search.
- Layout-ready card: now the Quick layout panel's compact size (see the tool panel comment below). `/layout`, then Enter (Aim). It shows L Edit, Esc Exit and RMB Place as key caps, nothing clickable.
- Place Tool page: `/place` (also "Place Tool overlay" in the screen picker). The full place Tool panel over play: Outside | Inside | Placing, the Game's Templates, Turn, the outcome and the size and Voxel total, with the ghost in the World. Prompt bar: ↑ ↓ Template, R Turn, V View, Enter Aim, Esc Exit. `/structures` is unchanged and still shares its stage code, but K from play now opens the place Tool panel.
- Ticket 13's behaviour: a press outside any open popup (Mods and Biomes
  filter and sort, World card menus, Settings and other selectors, the Map
  popover) closes it and is swallowed, so it presses nothing beneath. One
  capture-phase rule in `interaction.js`; a popup's own button still toggles
  it and Esc still closes it.
- The screen picker also lists Structures (`/structures`) and shows it there.

- Biomes panel, after review: the climate map comes first and the
  Temperature, Humidity and Caves sliders sit below it; Sky and Fog are one
  line each; the panel's body scrolls at 1280×720 while Reset and Done stay
  fixed. The selected Biome row takes the selected background and border, not
  only its left edge.

- Biomes panel Ground, after review: the climate map is 2:1. Surface, Soil and
  Rock are dropdowns whose field and options show the Material's colour swatch;
  the open list has a search field that keeps focus and filters as you type
  (arrows move, Enter picks, Esc closes the list only). The mock lists 16
  Materials (`GROUND` in `mockup/preview.js`, which the dioramas also read).
  `customSelect(…, { search })` adds the search; a swatch comes from an option's
  `swatch`. Esc with any dropdown open no longer also closes the Biome panel.
- A Biome's default Sky is `#87ceeb` and Fog `#ffffff`. A Game's Biome that
  declares its own keeps them (Tidewater, Ember, Lumen, Meadow palettes);
  Terra's declare none, so Terra shows the defaults, and Reset and Reset all
  restore them.

- Settings › Display opens with a Window group: Window mode cycles Windowed,
  Borderless fullscreen and Fullscreen (default Windowed) and, as a display
  change, opens the keep-or-revert prompt (ticket 21).
- Sticky list toolbars (ticket 19): the search line and the select-all row sit
  in one `.list-toolbar` that stays at the top of the scrolling list, opaque so
  rows never show through. Pages: the Mods step (`/worlds/new/mods`), the Biomes
  step (`/worlds/new/biomes`, also beside the open Biome panel), the Worlds
  search (shown past 8 Worlds), and the Map legend's Waypoints/Explored tabs
  (`/map`). `/mods` has no toolbar and the `/inventory` catalogue grid already
  scrolls under its search. Focus scrolls a row clear of the toolbar
  (`scroll-padding-top`).

- Toasts study (`/toasts`, also "Toasts (study)" in the screen picker): one
  toast component (`notify(text, tone)` in `mockup/toasts.js`, styles in
  `mockup/toasts.css`) for every screen. The study panel switches the backdrop
  (Worlds with its prompt bar, or the /play HUD in Survival with Compass and
  minimap on) and the position (bottom centre, top centre, top right, bottom
  left; defaults top centre in game, bottom centre in menus), and fires info,
  done, refused, an 8 s error, a repeat (×2, timer restarts) and three at once
  (at most 3, newest nearest the anchor edge). Dark notice-card panel, icon and
  left edge by tone (neutral, green, amber), fade with an 8 px move away from the
  anchor edge, 3 s; fade only under reduced motion. `toast()` and the engine
  mirror `notice()` both go through it at the screen's default position; the old
  `#toast` and `#notice-card` are gone. At 1280×720 with three toasts, bottom
  centre in game reaches the reticle; top right in menus and top centre in menus
  sit over the Worlds sheet's top edge; nothing touches the HUD or prompt bar.

- Settings help strip (`/settings/behavior` and every category): the boxed help column is gone, so the rows take the sheet's full width. A strip along the bottom of the sheet, above the prompt bar and outside the scrolling rows, shows the focused or hovered setting's name in bold, its explanation muted, and a note on the current value where it has one (`VALUE_NOTES` in `mockup/settings.js`: Sprint speed, Removal and Crouch activation, Window mode, Display sync, Frame rate limit, Anti-aliasing, Render scale). With no row focused it shows the category's one-line description (`CATEGORY_HELP`). Rows that had no explanation got one. `frame({ strip })` puts it after the body.

- Settings help strip look: the strip is a band in the side rail's tone (`rgb(10 11 12 / 0.9)`) across the foot of the sheet, keeping its 1 px top edge, with 6 px more padding above and below. The setting's name is bold at 15 px, the explanation stays muted, and the value note is in `--leaf` with the value bold (**Walk**: no sprint.), split at the first `: ` of the `VALUE_NOTES` text. Checked at 1280×720 in all seven categories.

- Esc leaves tools in play: each press takes one step, in this order: it cancels a pinned structure preview, then puts the place Tool away, then leaves Quick layout mode, and only then opens the pause menu. Backspace still does the first three. Esc on `/layout` and `/place` still exits. The Layout-ready card reads `Esc Exit`. The place Tool HUD card and the structure list footer now say Esc or K. The play prompt bar never named an exit key, so it is unchanged. Each step was checked in headless Chrome.
- Biomes panel Atmosphere colour picker: `/worlds/new/biomes`, a biome's
  pencil, then Sky or Fog. The native colour input is gone. The closed field is
  a swatch plus its hex value, one control; Click, Enter or Space (A) opens a
  popup with 8 presets (sky blue, white, dusk orange, overcast grey, night blue,
  jungle haze, desert sand, ember red), a saturation/brightness square, a hue
  slider and a hex field. Arrows move within the focused part (Shift for bigger
  steps), Tab and Shift+Tab (LB/RB) move between parts, Enter picks. Every
  change applies live, so the diorama redraws as you go: Esc (B) closes and
  restores the colour the popup opened with, while a click outside closes and
  keeps it. Clicking a preset picks it and closes. It opens above its row when
  it would run past the panel body's bottom, as Sky and Fog both do at
  1280×720. `colourField(label, bind, value)` in `mockup/colour-picker.js`;
  `placePopup` in `controls.js` now places both it and the dropdowns.

- One panel per tool (replaces the Esc-steps note above): Quick layout and the place Tool each have one panel docked top right while the tool is in hand. It is full size while choosing (`/layout`, `/place`, over play with the hotbar and the ghost; no crosshair or hand) and compact while aiming in play, in the same spot with the same title row and key-cap row. The compact panel shows the selection (shape, Voxel count and size, or the Template, facing, size and Voxel count), the status line, and L/K Edit, Esc Exit, RMB Place. The full panel has an Aim button (Enter) and L/K Aim, Esc Exit. L or K opens a tool and toggles full and compact; Enter or Aim collapses; Esc or Backspace leaves from either size, except that with a preview pinned the first Esc cancels it (the caps read G Build, Esc Cancel, K Edit, and so does the reticle hint). Building a Structure (G near the pin, or Simulate's "Build the Structure (G)") unselects the Template and reopens the full panel with "Pick the next Template"; Enter and K do nothing until one is picked, and ↑ ↓ picks the next. Quick layout keeps its layout: RMB says "Plane laid out · 16 Voxels." and stays in aim. L does nothing with the place Tool in hand, and K does nothing with Quick layout on. The separate Layout-ready and place Tool HUD cards (`layoutStatus`, `structureHud`) are gone, and the minimap hides while either panel shows. Walked in headless Chrome at 1280×720.

- Inherited Sky and Fog: in a Game where other Biomes declare colours, a Biome with none of its own shows its Sky and Fog fields empty with the muted placeholder "Same as the land around it" and a swatch of the other turned-on Biomes' declared colours, blended nearest-in-climate first as the Client's `land_around` does; its diorama uses that blend. Sky and Fog are one pair, as in the Client: picking either gives the Biome its own pair, the picked colour plus the other fixed at the blend it was showing; Esc in the picker puts both back to the placeholder, and Reset clears both. Only when no Biome in the Game declares colours does a colourless one show #87CEEB and #FFFFFF. Tidewater's Cold Shallows is now colourless to show it. The placeholder is one 12 px line and the Atmosphere rows have a narrower label column, because a two-line field shrinks when a colour is picked, the panel re-clamps its scroll and the picker closed. Checked in headless Chrome at 1280×720.

- Voxel size on the World step is a three-option segmented control like Game mode above it (1 m, 0.5 m, 0.25 m), with one line under it from the Client's `VOXEL_SIZES` saying what the chosen size costs ("1 m · default"). Left and Right pick the next size and leave the row at either end; there is no dropdown. The Client's ◀ value ▶ `voxel_selector` is to change to match. Edit World's fixed summary still reads "Voxel size 0.5 m". Checked in headless Chrome at 1280×720.

- Settings › Audio (`/settings/audio`): Sound mix adds an Interface slider (menu clicks and other menu sounds, apart from Effects; 0 mutes them; default 100). The "Fine-tune sound effects" disclosure is gone: an always-shown Sound effects section, headed like Sound mix with the note "Each is relative to Effects; 0 mutes it.", holds Footsteps through Hurt & death as sliders, each with its line in the help strip. Reset on Audio also resets Interface. `group(name, note)` in `settings.js` renders the note (`.section-note`). Settings › Controller adds "Rumble on menu press", On/Off, default On. Checked in headless Chrome at 1280×720.

- Create World reads in the background, as the Client now does: Next out of the Game step (or a rail jump past it) shows the next step at once and simulates a 400 ms read of the Game and its Biomes; Next out of Mods into Biomes or Review reads the Biomes. Meanwhile the World step's Climate scale shows the Client's muted "Reading the Game…" in place of its two fields, the Biomes step and Review's Biomes section show "Reading the Biomes the Game and Mods declare…" (Review drops its Biomes count and its Climate scale row, as the Client does), and Create World and Ctrl+Enter are disabled; then the content appears without moving focus. A dashed "Prototype · hold “Reading…”" button at the sheet's top right holds the state for inspection. Checked in headless Chrome at 1280×720.
