# 23: The UI matches the prototype

Status: ready-for-agent

Blocked by: 03, 04, 11, 13, 14, 16, 17, 18, 19

Spec: [Phase 15](../spec.md).

## What to build

Every screen this spec added or restyled matches the approved prototype one to one: theme, colours, fonts and weights, sizes, spacing, layout, borders and radii, states (hover, focus, selected, disabled) and copy. The screens are the title screen, Worlds, Mods, Create World (World, Mods, Biomes, Review), Edit World, the in-game Menu, Settings in every category, the Inventory with the Catalogue, the HUD (hotbar, held item name, vitals tiles, pickup feed, notice card) and the death screen. Each engine screen is compared with the same prototype route at the same window size, from a dev screenshot of the engine beside a browser screenshot of the prototype. Where the engine cannot do what the CSS does (a backdrop blur, a font feature), the closest engine equivalent is used and recorded in this ticket. The prototype's own chrome (the prototype bar, the Simulate panel, the Scores link) is not part of the design.

## Acceptance criteria

- [ ] Every listed screen has a side-by-side engine and prototype screenshot at the same size, and each difference is fixed or recorded here with its reason.
- [ ] Colours, fonts and spacing come from shared theme values, not per-screen literals.
- [ ] Every screen keeps its existing tests green.

## Comments

- Maintainer, 2026-11-06: the menus' theme, colours, fonts and layout do not yet match the mockups one to one.
- Fix pass A (theme), 2026-11-06: the engine's shared values live in `crates/tesserae/src/client/theme.rs`, taken from the prototype's final cascade (controls.css and polish.css over refinements.css over style.css). Where the engine cannot do what the CSS does:
  - **Backdrop blur** (`.home-actions`, `.overlay`, the Menu's and Settings' blurred landscape): Bevy UI has no backdrop filter. Panels use the opaque #0b0b0c fill the prototype already gives them, which hides any blur; dialog overlays are the flat #090a0c at 0.70 without the 4 px blur.
  - **Filters on the scenery or the World** (`saturate`, `brightness`, `grayscale` under menus and the death screen): the engine cannot filter the rendered World from the UI. Menus and Settings lay the prototype's `.shade` gradients over it (#192228 at 0.48 → 0.06 left to right, 0.24 → 0 bottom to top); the launcher keeps the flat #242c32 scene colour under the same gradients until it has scenery.
  - **Inset box-shadow** (the 1 px #ffffff1f ring inside every framed panel and the notice card): Bevy's `BoxShadow` has no inset, so the ring is a child node filling the padding box with a 1 px border, drawn under the panel's own children as CSS paints it.
  - **Focus outline**: drawn with Bevy's `Outline` (3 px #e2d5ab at 3 px offset; 2 px #f3f6ec at 2 px for checkboxes), which matches `outline` and `outline-offset`.
  - **Native range input**: the Settings slider is drawn from Chrome's look: a 6 px #efefef track with the sage fill to the left of a 16 px sage thumb.
  - **Dashed border** on a disabled field: Bevy UI borders are solid, so a locked field keeps the #4d4f55 edge solid.
  - **Body font**: the prototype renders the system UI font (SF Pro on macOS), which cannot be bundled. The maintainer chose Inter, the prototype's first family: Inter 4.1's variable font, under the SIL OFL 1.1 (ADR-0005 amended), so the prototype's 450/550/650 weights render exactly. Aileron SemiBold stays for the wordmark. Inter has ✓, ⌄, ←, → and ↗; it has no ⚙, which needs an icon.
- Fix pass C (Menu and Settings), 2026-11-06: what still differs from the prototype, and why:
  - **Biome in "You are here"**: the Server now sends the Biome at the Player's Eye (`HereBiome`), and the block shows only while one is known, so a World without Biomes has none.
  - **World details**: the Server reports no Generator version or climate sizes, so those lines are left out, and the Game shows without its version, as the prototype words it.
  - **Menu panel position**: the prototype's "Single-player prototype" notice is prototype chrome and is not ported, so the shorter panel centres a little lower.
  - **Settings applied**: the engine keeps its top-right notice card; it has no bottom toast for menus.
  - **Stronger UI contrast**: the engine has no such setting, so Accessibility has no row for it.
  - **Defaults, slider ranges and options**: the prototype now uses the engine's (Toggle sprint, 0.25–3 sensitivities, SMAA, no 50 % render scale, a 1000 cap, no 90), since they are what the profile validates. The same goes for the action names, key labels and binding order, which the prototype already used for the rows it added.
  - **Text size hint**: kept as the engine's, "100% is standard; 200% doubles…", because the prototype's "1 is standard" contradicts its own percent dropdown; the prototype now says the same.
  - **Restore defaults**: the engine resets the Controller page apart from the rest, so the dialog says which; the prototype now does too.
  - **Clearing a binding**: the prototype's rows have no ×, so Clear binding moved into the capture dialog; the prototype has it too. Capture takes every key, so clearing needs the pointer.
  - **Controller rows**: an alternative pad binding shows only once one is bound, as the prototype shows one button per action.
  - **Quick reference**: a session toggle, off by default, that lists the HUD keys under the page; the prototype's toggle shows nothing.
  - **Key bindings search**: it stays above the scrolling body, so a page rebuild cannot take its focus; at the top of the page it lands where the prototype's does.
  - **Sheet header**: 5 px taller than the prototype's, because the eyebrow's line box is 1.5 of its size where the browser's normal line height is about 1.2; it is the shared `cinematic::sheet_header`, so every sheet shows it.
  - **Hint wrapping**: Inter at 14 px is a little wider than the prototype's system font, so a few hints near the 320 px measure wrap one word earlier.
  - **World details overlay**: without a backdrop blur, the Menu under the 0.70 overlay reads clearer than in the prototype.

