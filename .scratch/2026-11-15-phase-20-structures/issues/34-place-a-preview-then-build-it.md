# 34: Place a preview first, then build it or cancel it

Status: done

Blocked by: 11, 29

Spec: [Phase 20](../spec.md).

## What to build

Play-testing asked for a two-step place Tool: "preview of the placement of
template should be ghost of actual template. we should be able to place a
ghost/preview of template and then move away to look at it before commit to
build it. so it is 2 step process. place preview, then we can select it to
cancel it or build it".

Decided 2026-11-16 by the user: one preview, the Player's own.

- **The ghost is the Template itself:** each cell drawn in its own Material and
  Shape, translucent, and red where placing would be refused, rather than
  tinted only by what placing would do.
- **Pin:** with the place Tool in hand, Place (right mouse) pins the ghost
  where it is aimed. It stays there as the Player walks away. The tool's turn
  still turns it.
- **Build or cancel:** aimed at the pinned ghost, Place builds it (the Server
  judges and lands it as today, one undo group) and Backspace cancels it.
  Elsewhere Backspace still puts the Tool away.
- **The Player's own:** the pinned ghost is held by the Client and never
  saved; other Players do not see it, and it goes when the World closes or the
  Tool is put away. The Server stays the only writer: nothing reaches the
  World until Build.
- The pinned ghost keeps asking the Server what placing would do, so its red
  cells follow the World.
- The mockup changes first: the HUD card says what Place and Backspace do with
  a ghost pinned.

## Acceptance criteria

- [x] The mockup shows the pinned ghost and its keys.
- [x] Place pins the ghost and sends no Structure intent; aimed at the pinned
      ghost, Place sends it, tested through the Client.
- [x] Backspace aimed at the pinned ghost cancels it; elsewhere it puts the
      Tool away, tested.
- [x] The ghost draws the Template's own Materials, red where refused.
- [x] Nothing is written to the World before Build, and Build lands whole or
      not at all as before.

## Comments

Mockup: the Placing view draws each cell in its Material's texture,
translucent, red where blocked; Right mouse (and "Place here") pins, and with
a preview pinned Right mouse builds and Backspace cancels; the card and the
list's footer say so.

Server: `StructureIntent::pinned`, the anchor of a pinned preview, which skips
the ray: both the aim answer and the placement take it as their anchor, and
refuse it beyond `PIN_REACH_M` (64 m) from the Eye. `StructureGhost::voxels`
carries what each cell would hold.

Client: `PlaceTool::pinned` holds the anchor and the heading it was aimed
along. `intercept`: Place with nothing pinned pins the fresh answer's anchor
(`structure::pin`, also behind the list's "Place here") and sends nothing;
with a preview pinned and the crosshair on its box (`aimed_at`), Place sends
the pinned intent and Backspace cancels it (the clear key is then taken, so
the Tool stays in hand); putting the Tool down drops the pin. The ghost hull
carries each cell's linear Material colour from `GhostPalette` (the Server's
`MaterialPalette`) at 0.7 opacity; refused cells keep the red tint.

Decided while building:

- Pinning again elsewhere does nothing while a preview is pinned; cancel it
  first. A second pin would need the Server to answer the live aim and the
  pinned one together.
- The 64 m pin reach replaces the 8 m Edit reach for a pinned preview, judged
  and built alike: the anchor was in reach when pinned, and a Player walks
  round it to look.

Tests: `a_pinned_preview_is_built_at_its_anchor_from_where_the_player_now_stands`,
`a_pinned_preview_further_than_its_reach_is_refused`,
`a_pinned_aim_answers_its_anchor_wherever_the_player_looks_and_the_ghost_names_its_voxels`
(Server); `place_pins_a_preview_and_aimed_at_it_place_builds_it`,
`the_clear_key_aimed_at_the_pinned_preview_cancels_it_and_is_taken`,
`the_ghost_is_drawn_in_its_materials_colours_and_a_refused_cell_in_red`
(Client). Checked in play: the cottage's ghost in its own Materials.
