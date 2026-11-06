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
- Fix pass E (HUD and death screen), 2026-11-06: play and death show no footer, as decided, so the prototype hides it there and both put the hotbar 12 px above the edge. Worlds open on the hotbar slot last chosen, kept by the Client in `user-profile.slot.json`. Refusals are sentences ("Nothing to place in hotbar slot 1."); the Undone and Redone notices are gone. Where the engine cannot do what the CSS does:
  - **Filters on the World under the death screen** (`grayscale(.75) brightness(.5) blur(1px)`): the camera's colour grading takes the World's saturation to 0.25 while dead. Brightness is a black layer at 0.78 under the red scrim: CSS halves sRGB values and Bevy UI blends in linear light, where that is 1 − 0.5^2.2, and unlike grading it also reaches the sky. Measured against the prototype, the sky lands at ×0.41 (prototype ×0.40) and mid terrain at ×0.35 (×0.39). There is no blur.
  - **Inset shadows on hotbar slots**: the 3 px dark and 2 px light bevel and the 1 px rim are child nodes with per-side borders; the selected slot's inner 10 px glow is a radial gradient child fading from 60 % to the edge.
  - **Blurred text shadows** (held name `0 1px 3px, 0 0 8px`, stack counts `0 0 3px`, the Inventory-full notice `0 1px 2px`): Bevy's `TextShadow` has one hard offset and no blur, so each keeps its offset in black and drops the blur.
  - **`ui-monospace`** on stack counts, slot numbers, feed counts and Respawn's "Enter": no monospace face is bundled, so they use Inter at the same size and weight.
  - **The feed row's fade-in**: rows slide in 16 px over 0.18 s but do not fade, because the Item tiles are 3D previews that cannot take an opacity. Rows leave at 3 s with no fade, as in the prototype.
  - **The prototype's reticle offset**: its "+" is anchored by its top-left at the centre, about (+8, +17) px off; the engine keeps it centred.
- Fix pass E, not done, 2026-11-06: F3's streaming readout stays `dev` only. Shipping it needs the Server's `streaming::stats` module and `cost::collecting`, both `dev` or `profiling` only, published in every build, which is a streaming change outside this pass. The undo refund that drops at the feet ("No room: 1 {Item} dropped at your feet.") and the item names in "Can’t undo: putting the {Material} back…" need the Server to say what it dropped or needed; the engine keeps "Putting that back needs an Item you don’t hold." The air row still shows only once air drops below full, not on the first frame under water.
