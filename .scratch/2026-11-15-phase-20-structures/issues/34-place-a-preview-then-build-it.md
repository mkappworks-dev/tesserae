# 34: Place a preview first, then build it or cancel it

Status: ready-for-agent

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
  where it is aimed. It stays there as the Player walks away; pinning again
  moves it. The tool's turn still turns it.
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

- [ ] The mockup shows the pinned ghost and its keys.
- [ ] Place pins the ghost and sends no Structure intent; aimed at the pinned
      ghost, Place sends it, tested through the Client.
- [ ] Backspace aimed at the pinned ghost cancels it; elsewhere it puts the
      Tool away, tested.
- [ ] The ghost draws the Template's own Materials, red where refused.
- [ ] Nothing is written to the World before Build, and Build lands whole or
      not at all as before.

## Comments
