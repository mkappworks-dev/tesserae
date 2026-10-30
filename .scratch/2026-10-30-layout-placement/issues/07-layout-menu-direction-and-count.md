# 07: Layout menu with six-way direction and count

**What to build:** turning Layout mode on opens an overlay menu, an input
context like the Catalogue, navigable by mouse, keyboard and pad. It sets the
mode, the shape, and for direction-and-count, per axis one of six directions
and a count by stepper or a typed number on a keyboard. The camera no longer
picks a direction. Closing it returns to play with the mode on and the
preview showing.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

Spec: [layout placement](../spec.md), amendment 2026-10-30.

- [ ] The Layout key opens the menu and turns the mode on; closing the menu
      keeps the mode on, and the Layout key turns it off.
- [ ] Directions are +X, -X, up, down, +Z, -Z with no two on one dimension.
- [ ] A count can be stepped on every device and typed on a keyboard; a count
      over the 256 cap is refused with feedback.
- [ ] The Material picker and gap move into this menu, with the gap in metres.
- [ ] Menu, focus and device changes cancel a pending commit; nothing is placed
      by opening or closing the menu.
- [ ] The rotate-axes and reverse actions are removed; count and shape keys
      stay.
- [ ] Mode, shape, counts, gap and directions persist; an older profile loads
      with defaults.
