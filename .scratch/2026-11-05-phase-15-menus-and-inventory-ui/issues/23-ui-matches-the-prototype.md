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
  - **Glyphs**: Aileron has no ✓, ⌄, →, ←, ↗ or ⚙; the checkbox and dropdown draw a check and a chevron from the engine's icon atlas, as the Reset icon already was.
  - **Body font** (open): the prototype renders the system UI font (SF Pro on macOS) everywhere but the wordmark. Bundling Inter, its first family, would put an OFL font in `assets/`, which ADR-0005 keeps CC0; Bevy's `system_font_discovery` would match on macOS but varies by platform and links fontconfig on Linux. Until that is decided, `theme::BODY_FAMILY` stays Aileron.
