# 09: Layout menu as an in-game overlay, and ghost-block preview

**What to build:** replace the Layout page in the settings with a panel docked
over the live game, with its own input context, and draw the preview as ghost
blocks instead of outlines.

**Blocked by:** None (can start immediately)

**Status:** done

Spec: [layout placement](../spec.md), amendment 2026-10-30 (2).

- [x] The Layout key opens the menu and turns the mode on; while the mode is on
      it opens and closes the menu, and Escape closes it.
- [x] The cursor is released while it is open, the pad navigates it, and the
      World stays drawn behind it.
- [x] Choices apply as they are made and are saved when it closes; unchanged
      choices write nothing.
- [x] Typing digits sets the active axis's count; steppers work on every
      device; a count over the cap is refused with a note.
- [x] The Layout page and its operations are gone from the settings.
- [x] Each preview cell is a ghost of the block that would land, tinted
      orange out of reach and red blocked; the single ghost is hidden while
      the mode is on.

## Comments

### 2026-10-30: implemented

The panel is built with the same buttons as the other menus, so the pad's
focus navigation works on it once its context is listed. Rebuilding the panel
on each change keeps focus on the same control. The ghosts reuse the held
model's mesh and the single ghost's translucent material, rebuilt when the
held block or the Material choice changes. The panel was checked from an
offscreen capture, not on a physical pad.
