# 08: Point-to-point mode

**What to build:** the point-to-point choice in the Layout menu and its play
flow. Place sets A, Place sets B, Place commits; Remove steps back a point and
then cancels. A and B come from the Server's aim answer, the preview shows the
span with an unset B following the aim, and the menu shows the span's size.

**Blocked by:** 06, 07

**Status:** ready-for-agent

Spec: [layout placement](../spec.md), amendment 2026-10-30.

- [ ] The three Place presses and the Remove steps behave as described, on
      keyboard and pad, with no typing.
- [ ] A and B must each be within reach of the Eye; the Player can walk
      between them.
- [ ] The preview, readout and menu show the span size and mark skipped cells.
- [ ] A span over 256 Voxels is refused with feedback and nothing is sent.
- [ ] Menu, focus and device changes drop the points and never commit.
