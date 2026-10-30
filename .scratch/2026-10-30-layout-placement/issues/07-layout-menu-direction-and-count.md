# 07: Layout menu with six-way direction and count

**What to build:** turning Layout mode on opens an overlay menu, an input
context like the Catalogue, navigable by mouse, keyboard and pad. It sets the
mode, the shape, and for direction-and-count, per axis one of six directions
and a count by stepper or a typed number on a keyboard. The camera no longer
picks a direction. Closing it returns to play with the mode on and the
preview showing.

**Blocked by:** None (can start immediately)

**Status:** done

Spec: [layout placement](../spec.md), amendment 2026-10-30.

- [x] The Layout key opens the menu and turns the mode on; closing the menu
      keeps the mode on, and the Layout key turns it off.
- [x] Directions are +X, -X, up, down, +Z, -Z with no two on one dimension.
- [x] A count can be stepped on every device and typed on a keyboard; a count
      over the 256 cap is refused with feedback.
- [x] The Material picker and gap move into this menu, with the gap in metres.
- [x] Menu, focus and device changes cancel a pending commit; nothing is placed
      by opening or closing the menu.
- [x] The rotate-axes and reverse actions are removed; count and shape keys
      stay.
- [x] Mode, shape, counts, gap and directions persist; an older profile loads
      with defaults.

## Comments

### 2026-10-30: implemented

The Layout menu is a Layout page in the settings overlay, which already has
mouse, keyboard and pad navigation, a cursor release, and draft and Apply. The
Layout key opens it when the mode turns on, and Back or Apply returns to play
with the mode still on. Two differences from the spec text: the overlay is
opaque, so the preview is not drawn behind it, and its choices take effect on
Apply like every other setting. Typing a count means choosing Type on an axis
and then entering digits; a pad uses the steppers. The page was checked from
an offscreen capture, not on a physical pad.

### 2026-10-30: superseded for the menu's home

The Layout page in the settings was replaced by an in-game overlay in
[ticket 09](09-layout-overlay-and-ghost-preview.md). Everything above about
directions, counts, typing and the camera no longer choosing a direction still
holds; the notes on the opaque overlay and Apply no longer do.
