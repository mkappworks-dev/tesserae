# 07: A menu picks anything declared into the hotbar

Status: done
Blocked by: 06

Spec: [decision 14](../spec.md).

## What to build

An engine Client menu, opened and closed with Tab and also closed by Escape.
While it is open, the cursor is free and mouse look and edit clicks are off;
closing it grabs the cursor again. It lists, from the content catalogue:
- solid Materials, then the Shapes for the chosen one;
- fluids;
- fire, when anything burns.

Each entry shows its full name and its placeholder colour. Choosing one puts it
in the selected hotbar slot, which then behaves as ticket 06 describes. While
the menu is open, clicking a slot in the hotbar selects it. The
Game has no say over the menu. It is engine UI drawn with Bevy's UI.

## Acceptance criteria

- [x] With the demo loaded, the menu offers every registered solid Material,
      including `demo:wood`, `demo:brick` and `demo:lamp`, in every registered
      Shape, the dev fence included in a `dev` build. It also offers
      `demo:water`, `demo:lava`, `demo:mud` and fire.
- [x] Given a catalogue with no fluid and no fuel, it offers the built-in solids
      and Shapes only, with no fluid section and no fire.
- [x] Choosing an entry fills the selected slot. The next click sends that
      slot's intent once the menu is closed.
- [x] Clicking a hotbar slot while the menu is open selects it, so the next
      entry chosen fills that slot.
- [x] No edit intent is sent and the camera does not turn while the menu is
      open.
- [x] Placing a declared solid chosen from the menu is accepted by the Server
      and reported to Mods as a solid Edit.
- [ ] A screenshot pass of the open menu with the demo loaded is reviewed with
      the user and then deleted. Screenshots are never committed. Screenshots
      in `/tmp/tesserae-p81-t07/` await the user's review.
- [x] CONTRIBUTING's controls table lists Tab.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

- **Module.** `client/menu.rs`, engine UI on Bevy's `ui_widgets::Button`
  (not the deprecated `Interaction`). Each button carries an `Entry`:
  `Material(id)` lists that solid's Shapes, `Solid { material, shape }`,
  `Fluid` and `Fire` fill the selected slot, `Slot(n)` selects a slot. One
  observer on `Activate` applies the choice. The menu is respawned whole
  when what it shows changes; the hotbar is compared by value, since
  `select_held` marks it changed every frame.
- **Layout.** Columns: Solids, "Shapes of <material>", Fluids (by
  declaration rank, only if any), Fire (only if anything burns). Under them
  a bar of the nine slots, the selected one outlined. Each entry is a
  swatch and the full name. The Shapes listed are the chosen Material's,
  else the held solid's, else the first solid's.
- **Colours** are the World's own: `ChunkMaterialExtension::default()`'s
  palette with the `MaterialPalette` message applied as `chunk_view` does,
  shown opaque; fire is the flame slot.
- **Input.** `toggle` runs before `send_intents`. Opening frees the cursor,
  so mouse look and edit clicks stop through the grab checks they already
  have; `camera::grab_cursor` does not run while the menu is open, so
  clicking an entry does not grab. Tab closes and grabs, and the next click
  edits.
- **Decision the spec leaves open:** Escape closes the menu but leaves the
  cursor free, as Escape does outside it; the next click grabs and edits
  nothing. Only Tab grabs on close.
- **Decision:** the bar is drawn only while the menu is open. The HUD text
  is unchanged apart from `[Tab] menu` in its key hint.
- **Dev fence:** the menu lists whatever Shapes the catalogue has. The
  Server registers `dev:fence` only with no Game (ticket 06), so with the
  demo it is absent and `demo:fence` is listed; the Client tests use a
  demo-shaped catalogue that includes it.
- **Capture:** `TESSERAE_SCREENSHOT_MENU` opens the menu for a `dev`
  capture, and an offscreen capture then makes the main camera the UI's,
  since the UI otherwise follows a window camera and none is left.
- **Tests.** `client/edit.rs`, headless, choices made by triggering
  `Activate` on the entry entity: the demo-shaped catalogue offers every
  solid (wood, brick, lamp) in every Shape (dev fence too), water, lava,
  mud and fire, each by full name; a dry one offers the six built-in solids
  and five Shapes, with no fluid or fire section; a chosen stair of wood
  fills slot 2 and the click after Tab closes sends it; Escape closes and
  leaves the cursor free; clicking slot 5 then fire fills slot 5; no intent
  and no camera turn while open, a turn once closed.
  `mods/runtime/tests.rs`: a Mod's wood stair placed by the menu's intent
  lands and a 0.3 Mod sees one solid Edit with it as `after`.
- Screenshot: `/tmp/tesserae-p81-t07/menu-demo.png`, the demo loaded,
  offscreen. Still lists `core:water` until ticket 09.
- Docs: CONTRIBUTING's controls table and a paragraph on the menu;
  `docs/architecture.md` lists `menu.rs`.
- `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass: tesserae
  365 (2 ignored), core 324, meshing 68, tesserae-mods 92 (3 ignored).
