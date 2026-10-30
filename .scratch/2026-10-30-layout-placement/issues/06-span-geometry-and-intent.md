# 06: Point-to-point span geometry and Server intent

**What to build:** a pure function that expands two opposite corner cells, a
shape and a gap into the ordered cells they span, and a Server intent that
takes the two corners and lays that span through the same per-cell judging,
tick budget and undo group as a directional layout. The preview answer covers
a span too, with an unset B taken from the aim.

**Blocked by:** None (can start immediately)

**Status:** done

Spec: [layout placement](../spec.md), amendment 2026-10-30.

- [x] A line spans the longest axis from A towards B, a rectangle the two
      longest and flat at A's level on the third, a box all three; ties prefer
      X, then Z, then up.
- [x] The gap steps from A towards B; a span over 256 Voxels is refused.
- [x] The Server judges every cell for reach, sight and occupancy; corners
      grant nothing, and out-of-reach or blocked cells are skipped.
- [x] One undo reverses the whole span, atomically as before.
- [x] Directional layouts behave exactly as before.

## Comments

### 2026-10-30: implemented

The span rides on `LayoutParams` as an optional pair of corners with the
shape's axis count, so one intent, one pending queue and one undo group serve
both ways of saying where Voxels go. An unset B takes the aim, which is how
the preview follows the cursor. The Client does not send spans yet; ticket 08
does. After the rebase onto main, layout cells also stay out of a physics body
through the shared landing code.
