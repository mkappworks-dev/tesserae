Status: done

# Menus that feel like a game

## Problem Statement

The launcher and in-game menus read as a web app, not a game: a header bar with
a logo and a settings cog, breadcrumbs ('MAIN MENU / WORLDS'), centred cards
with footer bars of Back buttons and status text, marketing copy, web arrow
glyphs, and nothing highlighted until the mouse moves.

Create World gives no sight of what a Game, a seed or the biome settings
produce, and a Mod appears as a name, so a Player cannot tell what a Mod adds
before enabling it.

Settings is a desktop preferences window (dropdowns, Apply and Restore buttons,
callouts). The Inventory is two app panels with an item-details sidebar of
buttons. The Map is a window with a toolbar. Quick layout is a form.

## Solution

Every menu follows game conventions: a title over the scenery, at most one main
button, and Back and the other actions as prompts in the bottom bar. Prompts are
clickable and answer their key. A menu opens with an item highlighted, the mouse
moves that same highlight, and a press shrinks and darkens the button at once
with a click sound.

The title screen is a logo, the word Tesserae and one menu with Continue first.
Worlds are save-slot tiles. Create World gains a Game step and shows live
terrain and biome previews. Mods and Games can ship a thumbnail.

Settings applies changes at once with ◀ value ▶ selectors and one help panel;
the Inventory uses slot tooltips and input actions; the Map is full screen;
Quick layout is a side panel with steppers; pause, death and loading screens
match.

The approved browser mockup is the design; each ticket implements its screens in
the Client.

## User Stories

1. As a Player, I want the title screen to open on Continue with my last World's
   name, so that I am one key from playing
2. As a Player, I want every menu to open with an item highlighted, so that
   arrows or the D-pad work at once
3. As a Player using a mouse, I want hovering to move the same highlight the
   keys move, so that there is only ever one
4. As a Player, I want Back and the other actions shown as key prompts in the
   bottom bar, so that I learn the keys while I use the mouse
5. As a Player, I want clicking a prompt to do the same as pressing its key, so
   that the bar works for both inputs
6. As a Player, I want a pressed button to shrink and darken at once and the
   action to happen without delay, so that menus feel responsive
7. As a Player, I want a click sound on every press and a short rumble on a
   controller, so that I feel the input land
8. As a Player, I want my Worlds as save-slot tiles with a terrain thumbnail, so
   that I recognise a World by sight
9. As a Player, I want New World as the first slot, so that creating a World is
   where I look first
10. As a Player, I want Enter to play, E to edit and Delete to delete the
    highlighted World, so that I need no per-card menu
11. As a Player with many Worlds, I want a search field once they no longer fit
    on one screen, so that I can find one by name
12. As a Player creating a World, I want to choose a Game first from a list with
    pictures, so that I know what kind of World I am making
13. As a Player, I want to trust the chosen Game when I leave the Game step, so
    that its previews can run
14. As a Player, I want a live terrain preview beside the seed field that
    redraws as I type, so that I can try seeds
15. As a Player, I want Randomize and Copy as icons inside the seed field, so
    that the form stays compact
16. As a Player, I want the voxel size and climate scale with the other World
    settings, marked fixed at creation, so that I know they cannot change later
17. As a Player, I want each Mod shown with a thumbnail and what it adds
    (Materials, Shapes, fluids), so that I can judge it before enabling it
18. As a Player, I want to filter Mods by category or selected-only and sort
    them, so that a long list stays manageable
19. As a Player, I want one checkbox to select or clear every Mod shown, so that
    bulk changes take one press
20. As a Player, I want Mod trust asked in a modal when I continue, so that the
    Mods step is not cluttered
21. As a Player, I want each biome shown with a small preview of its landscape,
    so that I can tell biomes apart
22. As a Player, I want to search, filter and sort biomes, so that I can find
    one in a large Game
23. As a Player, I want to edit a biome in a panel beside the list, so that the
    list stays in view
24. As a Player, I want to drag a biome on the climate map or move it with arrow
    keys, so that I can place it directly
25. As a Player, I want an edited biome's preview to redraw with my changes, so
    that I see their effect
26. As a Player, I want to reset one biome or all biomes, so that I can undo
    experiments
27. As a Player, I want Review to show each part of my World with a jump back to
    its step, so that I can fix a detail without starting over
28. As a Player editing a World, I want the same layout as Create World with the
    fixed settings summarised, so that both screens feel like one
29. As a Mod author, I want to ship a thumbnail in my mod.toml, so that Players
    see my Mod or Game as I intend
30. As a Mod author, I want validation to report a bad thumbnail by path and
    reason, so that I can fix it
31. As a Player, I want a Mod without a thumbnail to show its Material colours,
    so that it still has a picture
32. As a Player, I want Settings changes to apply the moment I make them, so
    that I see their effect without an Apply step
33. As a Player, I want a display change to revert after ten seconds unless I
    keep it, so that a bad setting cannot leave me stuck
34. As a Player, I want ◀ value ▶ selectors and On/Off toggles that left and
    right change, so that a controller can change every setting
35. As a Player, I want one help panel that explains the highlighted setting, so
    that rows stay short
36. As a Player, I want key bindings as key caps per input with categories on Q
    and E, so that rebinding reads like a game
37. As a Player, I want my Inventory as one panel with a tooltip on the slot I
    point at, so that item details do not need a sidebar
38. As a Player, I want click, Shift-click, right-click, Q and number keys to
    move, split, drop and send items to the hotbar, so that I do not hunt for
    buttons
39. As a Player, I want the tooltip to show which Shapes an item can be placed
    as, without choosing one there, so that Shape stays a choice made in the
    hand
40. As a Player, I want the Catalogue and recipes as icon grids with names in
    tooltips, so that I scan them by picture
41. As a Player, I want a Chest shown above my backpack in one panel, so that
    moving stacks is one Shift-click
42. As a Player, I want the Map full screen with a Waypoint and Explored legend
    on Q and E, so that it reads like a game map
43. As a Player, I want Quick layout as a side panel with count steppers and its
    keys in the prompt bar, so that I can set exact counts with either input
44. As a Player, I want the Layout-ready HUD card to show its keys as text with
    nothing to click, so that the HUD stays non-interactive
45. As a Player, I want the pause menu as a plain list with key caps, so that it
    matches the title screen
46. As a Player, I want World details to show the World's preview, fixed
    settings and biomes, so that I can recall how it was made
47. As a Player who died, I want the cause, where I fell and how long my items
    stay, with Respawn on Enter, so that I can decide quickly
48. As a Player, I want the loading screen full screen with the World's name, a
    tip and a slim bar, so that loading feels like part of the game

## Implementation Decisions

- One menu frame in the Client replaces the header bar, breadcrumbs and footer
  bars: a title, an optional one-line subtitle, the content and at most one main
  button. A side rail (Create World steps, Settings categories) is part of the
  frame, not a separate box.
- One prompt bar replaces the footer hints. Each screen declares its prompts as
  (key, label, act); a prompt with an act is clickable, and letter and Delete
  prompts fire on their key unless a text field has focus. Esc and Enter keep
  their existing handling.
- Focus first: entering a screen focuses its first item or a declared default,
  and pointer hover moves keyboard focus. Arrow-key and D-pad navigation is
  unchanged.
- Press feedback is a 97% scale and a slight darken, applied at once; the action
  is never delayed for the animation. A click sound plays on press and, on a
  controller, a short rumble.
- Create World has five steps: Game, World, Mods, Biomes, Review, with short
  titles and only the Game step carrying a subtitle.
- The Game's trust is asked when leaving the Game step, because the terrain
  preview runs the Game's Generator, which is Mod code. Mods are trusted at the
  Mods step in a modal, as today's trust rules require.
- A terrain preview is a new pure function of (Game, seed, climate scale,
  enabled biomes) returning a small top-down image from the same Generator the
  World uses, so it never disagrees with the World. It needs no Server and reads
  no World state, so it respects ADR-0002. It runs off the main thread and is
  redrawn on change.
- A World's thumbnail is rendered once at creation and stored with the save, not
  regenerated on each visit.
- A biome preview is drawn from the biome's own settings (surface, soil, rock,
  sky, fog) and redraws when they change.
- Climate scale moves from the Biomes step to the World step; it is a World-wide
  setting fixed at creation.
- mod.toml gains an optional thumbnail: a PNG path inside the Mod's folder,
  512×288, size-limited and validated like other Mod files. It is display data
  only, so it may be shown before the Mod is trusted. This extends the data-Mod
  contract and needs an ADR and a data-Mods documentation entry. A Mod without
  one shows one swatch per declared Material; a Game without one shows a neutral
  tile until it is trusted.
- A Mod's summary ('2 Materials · 2 Shapes · 1 fluid') is counted from its
  declaration, so the picture and the counts cannot disagree.
- Settings drop the draft/apply model: every change applies at once and Back
  asks nothing. Display changes (sync, frame-rate limit, anti-aliasing, render
  scale) open a keep-or-revert prompt with a ten-second countdown that reverts
  on timeout or dismissal.
- Key bindings show one cap per input column (Keyboard, Controller); mouse
  buttons print as LMB, RMB and MMB.
- Inventory Shape is read-only in the tooltip; the Shape is chosen in the hand
  while placing (Next Shape, Turn Shape), so the Inventory does not offer a
  second control for it.
- Quick layout counts are set with steppers in a side panel because exact
  numbers must be settable with a mouse; the same changes are on keys shown in
  the prompt bar.
- Esc does not leave the death screen, so its Return to Worlds button carries no
  key; the death screen has no prompt bar and Respawn shows its Enter cap on the
  button.
- The loading screen is full screen: the Game and World name at the lower left,
  a tip, and a slim progress bar along the bottom.
- The new frame and prompt bar are added beside the old chrome first; each
  screen moves over in its own ticket, and the old header, breadcrumbs and
  footers are deleted only when nothing uses them.
- Controller buttons map onto the same prompts: A is Enter, B is Esc, X takes a
  screen's secondary action and Y its destructive or reset action, and Start
  moves on in Create World. The prompt bar shows controller buttons after
  controller input and keys after a key press or mouse move.
- Enter activates the focused control. In Create World, Enter on the chosen Game
  or in a text field moves to the next step, and Ctrl+Enter (⌘+Enter) or Start
  moves on from anywhere; Edit World saves on Ctrl+Enter. Trust is never on
  Enter alone: its modal opens on Cancel.
- Next and Create World keep one position on every step: a stepped screen keeps
  one height and its content scrolls inside it.
- While the Player is the World's only Player, Edit World shows one Game mode
  that sets both the World's default and the Player's own mode.
- New World with these settings opens Create World on Review with everything
  filled in and a fresh name; it asks to save unsaved edits first, and the same
  Game and Mods need no new trust.
- Review, Edit World and World details show the seed as the number the World
  uses (as the core seed resolver makes it from words or digits), with a copy
  control.

## Testing Decisions

- A good test drives a screen the way a Player does (Acts, keys, pointer) and
  asserts what the screen shows, what is focused and which prompts appear; it
  does not inspect node trees or private state.
- Each screen is tested in its existing Bevy app test module (the launcher's
  Worlds tests, and the death, inventory window, map window and layout menu
  tests), using MinimalPlugins and the existing helpers that act, press keys and
  read shown text.
- The terrain preview's pure function is tested for determinism (same inputs,
  same image) and for agreeing with the Generator at sampled points, in the
  crate that owns generation, without Bevy.
- The thumbnail field is tested in the existing Mod-loading and validation
  tests: accepted path, a path outside the Mod's folder, a non-PNG, a wrong size
  and an oversized file, each reported by path and reason.
- Demo Game tests run on Terra only.

## Out of Scope

- The macOS Dock icon and menu bar, which have their own spec.
- Screenshots of play, or a World thumbnail that updates as the World is
  explored.
- Per-Mod detail pages.
- A Windows .ico or app bundles.
- Sound design beyond a single click and rumble on press.

## Further Notes

- Supersedes the earlier World and Mod previews draft, which covered only the
  previews.
- The mockup's thumbnails and biome dioramas are illustrations; the Client's
  previews come from the Generator and the biome's settings.
- Every screen keeps keyboard, controller and mouse working; the prompt bar is
  the place a Player learns the keys.

## Comments

Implemented on `feat/app-icon-and-game-menus` (tickets 02–17; 17 split from 02
for the click sound and rumble). After code review, the Trust page went back to
Back focused, per "Trust is never on Enter alone". A side-by-side comparison
with the mockup fixed ten visual differences. Left open for a decision:

- The Map opens with the last-added Waypoint selected; the mockup opens with
  nothing selected.
- The Layout-ready HUD card lists every key in a sentence; the mockup shows only
  "L Edit · ⌫ Exit" as caps.
- The Worlds and Trust sheets fill the screen; the mockup's fit their content.
- The Save-first dialog focuses Save and continue, where the mockup focuses
  Don't save, so Enter never discards edits.
