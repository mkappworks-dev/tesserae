# 05: Layout mini menu

**What to build:** a compact menu for shape, counts per axis, gap (Voxels with
metres beside it), signed axis toggles and a Material picker. Last choices
persist in the user profile.

**Blocked by:** 04

**Status:** done

Spec: [layout placement](../spec.md).

- [x] Every control works on mouse, keyboard and a pad with no typing.
- [x] The gap shows its metre value for the World's Voxel size.
- [x] The Material picker is a compact list over the catalogue's data and
      does not change the catalogue (tickets 05, 06 and 17 stay closed).
- [x] Choices persist across restarts; an older profile loads with defaults.
- [x] Applying or cancelling the menu never commits a layout.

## Comments

### 2026-10-30: implemented

The controls are a Layout group on the Behavior page of the settings, not the
docked in-world panel of the mockup. A docked panel cannot take a mouse click
while gameplay owns the cursor, nor a pad press while gameplay owns the pad,
so this gives all three devices the existing draft, Apply, focus and
navigation for free. The cost is that the menu hides the preview while it is
open; the hotkeys of tickets 03 and 04 steer the same saved values in play.
The Material choice is the session's and is not saved. A native visual review
of the page is still owed.
