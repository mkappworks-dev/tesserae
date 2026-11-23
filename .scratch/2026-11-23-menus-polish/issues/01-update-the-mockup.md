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
- Layout-ready card: `/layout`, then Enter (Build). The card shows L Edit,
  ⌫ Exit and RMB Place as key caps, nothing clickable.
- Place Tool page: `/place` (also "Place Tool overlay" in the screen picker).
  The Structures stage over the World with a Quick layout-style side panel:
  Outside | Inside | Placing, the Game's Templates, Turn, the outcome and the
  size and Voxel total. Prompt bar: ↑ ↓ Template, R Turn, V View, Enter Hold
  (back to play with the Tool), Esc Exit. `/structures` is unchanged and now
  shares its stage code; K from play still opens it.
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
