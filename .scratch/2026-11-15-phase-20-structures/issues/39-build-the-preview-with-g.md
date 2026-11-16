# 39: Pick, pin, then build with G or cancel with K or Backspace

Status: done

Blocked by: 29, 34

Spec: [Phase 20](../spec.md).

## What to build

Play-testing after ticket 34: "should be able to cancel the place tool with
either k or backspace. when a template is selected should be able to move the
preview template around and press right click place the preview. come close
you will see press g to build the preview and key k/backspace to cancel the
preview".

- **Pick:** choosing a Template in the list closes it into play, and the ghost
  follows the aim.
- **Pin:** right mouse pins the ghost where it is aimed; with one pinned it
  does nothing more.
- **Build or cancel:** within 8 m of the pinned preview, a prompt under the
  reticle reads `G Build · K / Backspace Cancel`. Use (G) builds it there; K or
  Backspace cancels it, from anywhere.
- **Put the Tool away:** holding the place Tool with no preview pinned, K or
  Backspace puts it away. K with any other hand opens the list as before.
- The mockup changes first, then the Client.

## Acceptance criteria

- [x] The mockup's keys, card and prompt follow the flow above.
- [x] Picking a Template closes the list, tested.
- [x] Right mouse pins and sends nothing; G within 8 m of the pinned preview
      sends the pinned intent; G further away sends nothing, tested.
- [x] K and Backspace cancel a pinned preview and keep the Tool; without one
      they put the Tool away, tested.
- [x] Left mouse on the pinned preview cancels it; elsewhere it does nothing,
      tested.
- [x] The prompt shows only near a pinned preview, tested.

## Comments

Mockup: `structuresPlace` only pins; `structuresBuild` builds on G;
`structurePrompt` shows `G Build K Cancel` under the reticle while a preview is
pinned (the browser takes the Player to be close); picking a Template
navigates back to play; K and Backspace cancel a pinned preview, else put the
Tool away; left mouse cancels a pinned one. Survival's G handler now yields
to the place Tool, as the Client's `intercept` takes Use.

Client: `intercept` (`client/structure.rs`) pins on Place with nothing pinned;
builds on Use within `BUILD_REACH_M` (the 8 m Edit reach) of the ghost's box
(`near`); cancels on Remove aimed at the box, or on the clear or Structures
key from anywhere, taking those keys so the Tool stays in hand; and sets
`PinPrompt`, which `use_prompt` shows as `G Build · K / Backspace Cancel`.
`structure_menu`: picking closes the list into play; the Structures key with
the place Tool in hand and the list shut puts the Tool away.

Tests: `place_pins_a_preview_and_near_it_use_builds_it`,
`the_clear_and_structures_keys_cancel_the_pinned_preview_from_anywhere_and_are_taken`,
`left_mouse_on_the_pinned_preview_cancels_it_and_elsewhere_does_nothing`,
`k_with_the_place_tool_in_hand_and_the_list_shut_puts_it_away`,
`picking_a_template_closes_the_list_into_play`. The prompt shows a frame after
the answer it is drawn from lands.
