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
- Fix pass D (Inventory, Catalogue, quick Layout), 2026-11-06: where the engine cannot do what the CSS does:
  - **Monospace text** (stack counts, slot numbers, the Catalogue's count and presets, Material ids, the axis letters): the interface bundles no monospace face, so these are set in Inter at the prototype's size and weight.
  - **Inset slot shading** (`inset 3px 3px #00000070`, `inset -2px -2px #ffffff0d`, the selected slot's inner glow): Bevy's `BoxShadow` has no inset, so a child with an uneven border draws the dark top-left and light bottom-right bands under the 1 px rim; the selected slot's 10 px inner glow is left out.
  - **Dashed outline** on a picked-up slot: Bevy UI borders are solid, so the rim turns #e7d9a8 and the Item fades to 0.6.
  - **Filters on the World** under the Inventory (`saturate(.8) brightness(.82)`): the scrim is deepened to #030405 at 0xdb, which lands the World at about the prototype's ×0.41 brightness without the desaturation.
  - **SVG icons** (the storage bag, Line, Plane and Box): drawn from bordered nodes, rotated and squashed for the diamonds. Inter has no ⇆, so an axis's way reads "East ↔".
  - **Editable counts**: the steppers' values are text, not inputs; digits typed while the Layout panel is open still set the active axis.
  - **In-play footer**: play has none, so the Inventory centres in the full 848 px (window top 104, not 87) and the Layout panel runs to 22 px from the bottom.
- Fix pass D, 2026-11-06: what still differs on purpose. The Layout panel keeps the live "In the World" box (from the prototype's engine parity additions) and hides its key hints as polish.css does; the closed-panel card sits top left in the prototype's look but has no Edit or Exit buttons, because play keeps the cursor grabbed, and it keeps the key hints. The Inventory shows its Shape choices only while a solid is held, since the Shape is the Player's choice for the held Material. The HUD hotbar and the cursor hint still draw under the Inventory (pass E's files).
