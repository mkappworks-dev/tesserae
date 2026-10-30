# Layout placement: lines, rectangles and boxes of Voxels

Status: grilled with the user 2026-10-30; tickets 01-05 are ready-for-agent.

Source baseline: `446e984`. Separate from
[ticket 29](../2026-10-27-gameplay-performance-and-controls/issues/29-repeat-while-held.md),
which only repeats single placements.

## Problem Statement

Placing many Voxels in a uniform pattern is not possible today. Each Place
snaps to wherever the Player aims, so a row, wall or floor of Voxels comes out
haphazard, and every Voxel costs a separate click. Players want a CAD-like way
to say "twelve Voxels, this direction, this spacing" and see it before it
happens.

## Solution

A Layout mode. The Player turns it on, aims at a spot, and a preview shows
exactly which Voxels would be placed, in which direction, and which are out of
reach. A small mini menu sets the shape (line, rectangle or box), the count on
each axis, the gap between Voxels, the direction and the Material. Place
commits the whole layout as one request; Remove or the toggle cancels. The
Server validates and applies it, and one undo reverses the whole layout.

## User Stories

1. As a builder, I want to switch Layout mode on and off with one action, so
   that normal single placement is untouched when I do not want it.
2. As a builder, I want a preview of every Voxel a layout would place, so that
   I see the result before committing.
3. As a builder, I want the preview to show the direction the layout grows, so
   that I do not place a wall the wrong way.
4. As a builder, I want to set the number of Voxels along a line, so that I
   can lay a fence row or a beam in one action.
5. As a builder, I want rectangles, so that I can lay a floor or wall.
6. As a builder, I want boxes, so that I can lay a solid block of Voxels.
7. As a builder, I want a gap between Voxels, so that I can space pillars or
   fence posts evenly.
8. As a builder, I want the gap shown in metres as well as Voxels, so that I
   can reason in real distance at any Voxel size.
9. As a builder, I want to flip the growth direction on each axis, so that I
   can build away from or towards me without re-aiming.
10. As a builder, I want the layout to anchor on the Voxel I aim at, so that
    it starts where I am looking.
11. As a builder, I want Voxels beyond my reach shown as unreachable and not
    placed, so that I know to move and repeat rather than wonder why some were
    missed.
12. As a builder, I want Voxels that would collide with solid terrain or the
    Player shown differently and skipped, so that a layout never fails as a
    whole for one blocked cell.
13. As a builder, I want to see how many Voxels will be placed, so that I know
    the size before I commit.
14. As a builder, I want a hard maximum layout size, so that a slip cannot
    flood the World.
15. As a builder, I want Place to commit and Remove or the toggle to cancel,
    so that the controls match what I already know.
16. As a builder, I want one undo to reverse a whole layout, so that a
    mistake costs one key press.
17. As a builder, I want redo to reapply the whole layout, so that undo is
    safe to try.
18. As a builder, I want undo refused cleanly if something I placed was since
    changed, so that undo never corrupts later work.
19. As a builder, I want a mini menu with Material, counts, gap and
    direction, so that I can configure a layout without leaving Layout mode.
20. As a builder, I want to change the Material in the menu, so that I do not
    have to leave Layout mode to swap hands.
21. As a builder, I want the count on the active axis adjustable by wheel or
    keys while aiming, so that I can size a layout quickly.
22. As a controller player, I want to enter, size, commit and cancel a layout
    with a pad and no typing, so that Layout mode is not keyboard-only.
23. As a keyboard player, I want every Layout action remappable, so that it
    fits my bindings and conflicts are warned.
24. As a builder, I want my last shape, counts and gap remembered, so that I
    do not reconfigure every session.
25. As a builder, I want the committed layout to appear as Voxels over a few
    ticks without hitching the game, so that large layouts stay smooth.
26. As a builder, I want a single message if some Voxels were skipped, so that
    I learn the cause without a toast per Voxel.
27. As a Game or Mod author, I want the Server to stay the only authority, so
    that Layout mode cannot be used to bypass reach or validation.

## Implementation Decisions

- Vocabulary: Voxel, Material, Shape, Edit, Eye, Server, Client as in
  CONTEXT.md. A **layout** is a request to place a set of Voxels; it is not an
  Edit by the Client. The Server's resulting changes are Edits. Gap, count and
  axis are metres-derived only for display.
- **Layout geometry is a pure function in `tesserae-core`**: anchor Voxel
  coordinate, orientation (a signed axis per layout axis), counts per axis,
  gap in Voxels -> ordered list of Voxel coordinates. No Bevy, no wgpu
  (invariant 3). Server and Client both call it so the preview cannot disagree
  with the result.
- Modes in scope: line (1 axis), rectangle (2 axes), box (3 axes). A box is a
  rectangle with a third count. Hard cap of **256 Voxels** per layout, equal
  to the existing pending-queue bound; the menu refuses counts that exceed it.
- Layouts land on the Voxel lattice. Gap is an integer number of Voxels (0 =
  adjacent). The menu shows the derived metre value using the World's Voxel
  size (ADR-0003); nothing is stored in Voxel units across Worlds with
  different Voxel sizes beyond the Voxel count itself.
- **Anchor** is the Server-resolved ghost cell of the current aim, the same
  answer a single Place uses. The layout grows from it along the chosen signed
  axes. The Client sends aim reports with the layout parameters; the Server
  answers with the cells that are acceptable, reach-clipped and which are not,
  so no preview reads World storage (existing rule, architecture.md).
- **One batched intent.** The Client sends a single layout intent carrying the
  aim ray, the held item, the shape, the counts, the gap and the signed axes.
  It never sends N placements and never contains cell lists the Server must
  trust: the Server expands the geometry itself from the anchor it resolves.
- **Server validation is per cell, partial by design.** Each cell is judged
  with the existing single-Place rules (within `EDIT_REACH_M` of the Eye,
  target empty, Chunk loaded and lit, not inside the Player). A failing cell
  is skipped, not fatal. The Client never mutates World state (ADR-0002).
- **Caps hold.** A layout is one pending item. It counts once against
  `MAX_PENDING`, but drains through the same `EDITS_PER_TICK` budget as single
  Edits (16 per tick), so a 256-Voxel layout lands over 16 ticks and cannot
  starve other intents. A partially applied layout keeps its place at the
  front of the queue. Reach is re-judged against the Eye when the intent
  arrives, and each cell is also checked when applied.
- **Grouped history.** Applied cells are recorded as one group entry in the
  undo journal. Undo and redo are atomic over the group and keep the existing
  Chunk Lifetime, settled-light and exact-Occupants guards; if any cell
  fails a guard, the whole undo is refused with the existing `HistoryChanged`
  message and nothing changes. The journal still holds at most 128 entries; a
  group counts as one entry and total recorded Voxels are bounded (proposed
  1024, oldest evicted first, to be fixed in ticket 02). This amends the
  one-Voxel-per-entry shape described in architecture.md; ticket 02 updates
  that paragraph. It also gives ticket 29 the grouping primitive it deferred.
- **Mods** see the same Edit events as for single Edits through the existing
  history path. No new WIT surface, no Mod Command (ADR-0001): a layout is
  Player input, not a Mod call.
- **Input.** A remappable `Layout` action toggles the mode. In Layout mode
  Place commits and Remove cancels; the toggle also cancels. Wheel (with a
  modifier) or step keys change the count on the active axis; a cycle action
  changes the active axis. The pad gets default bindings in the same ticket
  as the keyboard, using the existing pad binding set; menus use the existing
  focus navigation with sliders stepped by the d-pad. Gameplay input
  ownership and cancellation (menu, focus, device change cancels a pending
  layout) follow the existing semantic-action rules.
- **Preview** is drawn by the Client from the Server's answer: an outline per
  accepted cell in the held model's ghost style, unreachable and blocked cells
  in a distinct tint, a count readout, and an arrow for the growth direction.
  Hidden on item change, large turns or Eye displacement like the single
  ghost, and never drawn from World storage.
- **Mini menu** holds shape, counts, gap (Voxels with metres beside), signed
  axis toggles and a Material picker. The Material picker reuses the
  catalogue's data and tiles in a compact list; it does not redesign the
  catalogue (tickets 05, 06 and 17 stay closed). Choices persist in the user
  profile with defaults for older profiles.
- Fluids and fire: layout places only solids in this spec, as ticket 29 does.

### ADR and spec tensions

- No ADR is contradicted. ADR-0002 and ADR-0003 are honoured by the Server
  expansion and the metre display. ADR-0001 is unaffected.
- The undo journal's one-entry-per-Voxel shape (architecture.md) changes to
  support groups; this is a design amendment, not a contradiction.
- The Material picker overlaps the accepted catalogue scope of tickets 05 and
  06; it is limited to a compact pick list so as not to reopen them.

## Testing Decisions

- A good test drives external behaviour: a layout intent in, World state,
  rejections, history and preview answers out. No test reads private
  structure.
- **Server seam (highest):** the headless tick app used by `server/edit.rs`.
  Tests: cap of 256; 16-per-tick drain; skip of blocked, occupied and
  out-of-reach cells; reach judged from the Eye, not the camera; other
  intents are not starved; grouped undo/redo atomicity and refusal.
- **Core seam:** pure unit tests for line, rectangle and box geometry,
  signed axes, gap, the 256 cap, and Voxel-size independence of the Voxel
  list.
- **Client seam:** intent-trace tests for toggle, commit, cancel, count steps
  and cancellation on menu, focus and device change, at 60/144/1000 Hz render
  rates, plus pad-only completion of the journey.
- Prior art: `server/edit.rs` tests for pending, caps and history; the
  controller tests in ticket 07; the `command.rs` box cursor in `tesserae-mods`
  for shared box iteration ideas.

## Out of Scope

- Circles and a free sketch. They are later modes that reuse the layout
  geometry function and intent path; no tickets yet.
- Layouts across Chunks the Server has not loaded, and any Server-side
  streaming of layouts beyond 256 Voxels.
- Layout removal or fluids and fire layouts.
- Custom Shape orientation per cell beyond the held item's Orientation.
- Mod-declared layout modes or Game policy for layouts.
- Touch journeys (tickets 08-10).
- Grouping a held single-placement chain for ticket 29 (the primitive exists
  after ticket 02; adopting it is ticket 29's call).

## Further Notes

- Reach is the binding limit on real use: 8 m is only eight Voxels at the
  default size, so long lines are built in segments. Reach-clipping with a
  visible preview is the chosen answer rather than a longer reach.
- Open for ticket 02: the exact Voxel bound on the history journal and whether
  a skipped-cell summary is one toast or a status line.
