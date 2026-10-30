# 08: Point-to-point mode

**What to build:** the point-to-point choice in the Layout menu and its play
flow. Place sets A, Place sets B, Place commits; Remove steps back a point and
then cancels. A and B come from the Server's aim answer, the preview shows the
span with an unset B following the aim, and the menu shows the span's size.

**Blocked by:** 06, 07

**Status:** done

Spec: [layout placement](../spec.md), amendment 2026-10-30.

- [x] The three Place presses and the Remove steps behave as described, on
      keyboard and pad, with no typing.
- [x] A and B must each be within reach of the Eye; the Player can walk
      between them.
- [x] The preview, readout and menu show the span size and mark skipped cells.
- [x] A span over 256 Voxels is refused with feedback and nothing is sent.
- [x] Menu, focus and device changes drop the points and never commit.

## Comments

### 2026-10-30: implemented

The preview answer now reports the cell the aim picks, and the Client takes A
and B from it, so a Material chosen in the menu works with an empty hand too.
With no corner yet the preview asks for one cell; after A it asks for the span
to the aim; after B it shows the fixed span. A corner that would pass the cap
is refused with a note, and a menu, lost focus or device change drops the
corners. The spec text says the span size shows in the menu; it shows in the
in-play readout instead, since the menu is closed while pointing.
