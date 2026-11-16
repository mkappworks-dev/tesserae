# 11: The Client lists, turns and shows a Structure

Status: done

Blocked by: 01, 10

Spec: [Phase 20](../spec.md).

## What to build

Holding the `place` Tool, a Creative Player opens the Template list with Use,
turns the Structure with `R`, sees the ghost tinted per cell and places with
Place, with the HUD and refusals approved in ticket 01.

## Acceptance criteria

- [x] The Client matches the approved mockup.
- [x] The list shows every Template the Game and its Mods declare, with none
      hard-coded.
- [x] `R` turns by quarter turns and the ghost follows the aim.
- [x] A refusal from the Server shows as the alert the other Tools use.

## Comments

- Code: `crates/tesserae/src/client/structure.rs` (input, aim, ghost) and
  `client/structure_menu.rs` (Template list and HUD card), modelled on the
  Layout tool's `layout.rs` and `layout_menu.rs`.
- Use with the place Tool sends the Tool's Use as before; the list opens on
  the Server's `UseResult::TemplateList`, so Survival gets the
  `CreativeOnly` alert every Tool refusal uses. Place (right mouse) sends
  `StructureIntent`; its refusals arrive as `EditRejected` and show through
  the existing alert. Remove is taken while the Tool is held, so it sends no
  Use; there is nothing for "left mouse cancels" to cancel in play, so the
  hint leaves it out.
- `R` is a new action, `TurnStructure` ("Place Tool: turn the Structure",
  Building), rebindable in Settings. R was already Respawn, so the two
  share the key by default: `Action::may_share` lets that one pair through
  the default-collision test, the Settings conflict check and the profile
  loader, and with the place Tool in hand the turn cancels Respawn. A pad's
  Shape-turn button turns the Structure too.
- The ghost is drawn only from a fresh `StructureAnswer` (same Template and
  turn, aim within 2 m and about 20 degrees, under 0.5 s old). Tints are
  the mockup's: Places = Free, Replaces = ground, ReplacesFluid = water,
  Refused = Blocked; Carves is counted as ground and outlined (top faces,
  gizmo lines, at most 4,096 cells). Unchanged and waiting (NotLoaded,
  Busy) cells are counted as Free but not drawn, since air over air is
  Unchanged and would otherwise fill every room. Cells are merged into one
  mesh per tint and rebuilt only when the answer's cells change.
- The list's Templates are `ContentCatalogue::templates`, flat, named by
  `declared_title`, with size in metres and the full name. No kind or note
  is declared (ticket 10), so the mockup's groups (Homes, Trades...) and
  per-Template notes are not shown.
- Against the mockup: the list keeps the header, TEMPLATES · GAME · N,
  picks, TURN row with Facing, WHAT PLACING DOES legend, Authored / This
  World totals, outcome box and the footer (Place here, Put the Tool away).
  The HUD card keeps title, facing, size and Voxels, compact legend,
  outcome and key hints. Left out: the HUD's isometric thumbnail (the ghost
  is in the World itself) and its two buttons (the cursor is grabbed in
  play; G and right mouse do the same). Outcome texts for refusals are the
  Server's `RejectReason::message()` ("Nothing in reach.", "You are
  standing there.") rather than the mockup's wording.
- Not checked in the real game: no Game declares a Template yet, so the
  list would be empty. Declaring the `place` Tool in Terra and the Games'
  Templates is left to ticket 17. For its screenshots,
  `TESSERAE_SCREENSHOT_PLACE=1` holds the place Tool and `=list` also opens
  the list.
- The `expect(unused_imports)` / `expect(dead_code)` on `StructureCell` and
  `StructureGhost` are gone; `StructureGhost` is re-exported for tests only.
