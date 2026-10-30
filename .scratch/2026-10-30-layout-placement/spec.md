# Layout placement: lines, rectangles and boxes of Voxels

Status: grilled with the user 2026-10-30; tickets 01-08 are done (06-08 are
the amendment below).

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

## Amendment 2026-10-30: point to point and the Layout menu

Raised on review of PR 36: a layout that grows a fixed count from the aimed
cell, in a direction the camera picks, is not how a builder thinks. Two ways
to say where Voxels go replace it, chosen in a menu that opens with the mode.

### Behaviour

- Pressing the Layout key turns the mode on and opens the **Layout menu**, an
  overlay like the Catalogue: the cursor is released, and mouse, keyboard and
  pad navigate it. The World stays drawn behind it so the preview shows.
  Closing the menu returns to play with the mode still on; the Layout key turns
  the mode off.
- The menu holds a mode choice, the shape, and that mode's settings. The
  Material picker and the gap move here from the Behavior page.
- **Point to point.** In play, Place sets point A, Place sets point B, Place
  commits; Remove steps back one point and then cancels. A and B are the cells
  a single Place would pick, each within reach of the Eye, so the Player may
  walk between them like the measuring tool. A and B are opposite corners of
  an axis-aligned span, and the shape says how much of it fills:
  a line takes the longest axis, a rectangle the two longest and is flat at
  A's level on the third, a box all three. Ties prefer X, then Z, then up. The
  gap steps from A towards B; a span over 256 Voxels is refused with feedback.
- **Direction and count.** The camera no longer chooses a direction. For each
  axis the shape uses, the menu sets one of six directions (+X, -X, up, down,
  +Z, -Z, no two on one dimension) and a count by stepper, plus a typed number
  on a keyboard; a pad needs no typing. The anchor is the aimed cell, and
  Place commits.
- The count and shape keys stay as shortcuts; the rotate-axes and reverse
  keys go, since a direction is now picked outright.

### Decisions

- The Server still resolves and judges everything. A point-to-point intent
  carries the two corner cells, which are claims only: every cell of the span
  is judged for reach, sight and occupancy as before, and the 256 cap is
  checked from the corners. This refines the rule that a cell list is never
  trusted, since corners fix geometry and grant nothing.
- The Client takes A and B from the Server's aim answer (the ghost cell), so
  no new pick message is needed; the preview of the span while B is unset uses
  the aimed cell as a provisional B.
- The menu is a new input context beside the Catalogue and Settings. Typing a
  number uses the existing text-entry context.
- Layout preferences gain the mode and the six-way directions, and lose the
  camera-relative reversals. An older profile with the old fields loads with
  defaults for the new ones.

### Out of scope

- Three or four clicks to choose the plane (considered and declined); a key
  to swap the flat axis can follow if the automatic choice proves wrong.

## Amendment 2026-10-30 (2): the menu is an in-game overlay, the preview ghosts

Raised on review of the first pass at the menu: a page in the settings is not
the overlay the amendment above asked for, and an outline is not a preview of
what will be built.

- The Layout menu is a panel docked over the live game, not a page in the
  settings. It has its own input context beside the Catalogue and the settings:
  the cursor is released, the pad navigates it, gameplay input waits, and the
  World stays drawn behind it so the preview shows.
- Its choices apply as they are made and are saved when it closes. It has no
  draft or Apply.
- The Layout key opens it and turns the mode on; while the mode is on the key
  opens and closes it. Escape closes it, leaving the cursor free as the
  Catalogue does. Remove in play, and an Exit control, turn the mode off.
- The preview draws each cell as a ghost of the block that would land, in the
  held Material and Shape, with the single-placement ghost's look. A cell out
  of reach is tinted orange and a blocked one red.

## Amendment 2026-10-30 (3): a compact card, no Material, no corner panel

- The Material picker is gone: a layout lays the held block, and the Material
  of the hand is the one way to choose it.
- The menu is a compact card at the right edge, halfway down and only as tall
  as its content, so it never covers the crosshair or the preview. It is not a
  full-height strip.
- The information the corner panel showed (shape, counts and directions, the
  next step, what the aim would lay, the keys) is in the card. With the menu
  closed and the mode on, the card stays as a small status card with that
  text and no controls.

## Amendment 2026-10-30 (4): closer to Lay of the Land's flow

Asked for after comparing with Lay of the Land, whose placement tools were only
known from its store pages (Steam app 2776090). What follows is our reading of
the request, not a copy of that game, and needs the user's check.

- **Shapes stay as they are** (line, rectangle, box). Cylinder, cone and other
  round shapes are deferred.
- **Drag to pick.** In point to point, pressing Place on a cell sets A and
  holding it while the aim moves previews the span to the aim; releasing sets
  B. Click, click, click still works. A press that never moves picks A only.
- **Handles in the world.** A and B are drawn as markers on their cells, with
  the span's size along each axis (in Voxels, and in metres) written beside it,
  as the measuring tool shows its corners. They follow the preview.
- **Key prompts in the card.** The card names each key by its current binding
  for what it does now: set A, drag to B, commit, step back, cancel.
- **Not adopted:** layouts that ignore the Voxel lattice. A layout is Voxels at
  lattice cells in a World with one Voxel size (ADR-0003), so a span always
  lands on whole Voxels.

## Amendment 2026-10-30 (5): L opens and closes the menu, a tool key leaves

Raised on review: an "Exit layout mode" button is not how a Player expects to
leave, and the Layout key should work as it does for the measuring tool.

- The Layout key opens the menu, turning the mode on. Pressed again it closes
  the menu into play, the layout ready to place, and it opens again on each
  press after that. It never leaves the mode.
- The mode is left as the measuring tool is put away: the clear key (Backspace
  by default), or Remove once no corner is picked (left click, RT). Neither
  needs a button in the menu, and the menu has none.
- The card names those keys in its prompts.
- A card taller than the screen scrolls, with the wheel and with pad focus, and
  keeps its place when a change rebuilds it.

## Amendment 2026-10-30 (6): no shape in point to point, leaving, a calmer card

- Point to point has no shape. A and B are opposite corners of a box, and a
  dimension they share collapses, so one differing axis is a line, two a flat
  rectangle and three a box. Dragging in the direction wanted is the whole
  choice. This replaces the rule that the shape picks the longest axes. The
  shape control and the shape key do nothing there; Direction and count keeps
  them.
- The clear key (Backspace) leaves the mode from play and from the open menu,
  and so does the cross in the menu's corner. The Layout key and Escape only
  close the menu.
- The card is set in the app's own type and colours: a title with a chip for
  the way in use, option tracks as wide as their choices, one recessed group
  per axis with its direction, count and a typed number, the gap shown in
  metres, and a leaf-edged box for the shape, what the aim would lay and the
  keys.

## Amendment 2026-10-30 (7): A, B, then C

Raised on review: point to point was built as two corners spanning a box, but
what was meant is a flat rectangle from A and B, and a third point C for the
depth. This replaces the shapeless two-corner rule of amendment (6).

- Place sets A, Place sets B, and Place on C sets the depth and lays the
  layout. Holding Place on A and letting go on B still sets B.
- A and B span a flat rectangle on the plane they differ most in: the two
  longest differences, a tie going to X, then Z, then Y. The rectangle is flat
  in the remaining dimension, so a floor is flat in Y and a wall in X or Z.
- C gives the depth along that dimension: how far C lies from A along it, and
  to which side. Only that part of C is used. C level with A is no depth, so
  the rectangle stays flat.
- While B is unset the aim stands in for it, and while C is unset the aim
  stands in for C, so the preview follows the cursor through each step.
- A point that would pass the 256 Voxel cap is refused with a note, and
  nothing is sent.
- The ghost blocks are drawn a hair smaller than their cells, or the faces they
  share with the ground and each other flicker. The Client keeps more aim
  requests waiting for answers, so a fast turn does not drop them.

## Amendment 2026-10-30 (8): the card, status first

Settled from three mocked directions (a bottom bar, a side card with a
direction pad, and a status-first card); the side card with the status block of
the third was chosen.

- The card opens with the answer: how many Voxels the aim would lay, their size
  in Voxels and in metres, and what to do next. In point to point the A, B, C
  line sits there and fills in as the corners are picked. The block has no
  border, only a soft tint.
- Direction and count shows one axis open at a time: its direction, a stepper,
  a compass for the four horizontal ways and Up and Down in the right corner,
  level with the compass's top and bottom rows. The other axes are one folded
  row each, opened by choosing them. The open axis is the one the count keys
  and typed digits act on; there is no "type a number" control.
- Point to point shows no shape or axes, only its note, the way back and the
  gap.
- The gap is folded to one row until asked for.
- The keys are one muted line at the foot, each named by its first binding.
- The font has no arrows or triangles, so none are used; folded rows end in a
  plain ">".

## Amendment 2026-10-30 (9): three axis lines, C from the view line

Raised on review: a list of six directions and a shape choice is more than a
Player needs to say what to build.

- Direction and count is three fixed lines, X, Y and Z. Each has a length, a
  plus and a minus for which side it grows towards, and beside the letter only
  the name of the way chosen: east or west along X, up or down along Y, south
  or north along Z. A length of 1 leaves the axis out, so one axis above 1 is
  a line, two a flat rectangle or a wall, three a box. There is no shape
  choice and no six-way direction pick; this replaces both.
- The length keys and typed digits act on the line last touched. The reverse
  key flips the active axis, the axis key moves X, Y, Z and round, and on a pad
  the turn and empty-hand buttons do the same.
- The status block is a soft dark wash, not green, with the Voxel count and its
  word on one line, and the size line ends in what the lengths make.
- In point to point, C no longer needs a surface under the aim. It is the cell
  on the line through B along the dimension the rectangle is flat in, nearest
  to where the Player is looking, so a height can be set in open sky. A view
  that runs along that line, or away from it, sets nothing.
- A saved layout from before is not carried over: its fields are ignored and
  the layout starts from its defaults, the gap excepted.

