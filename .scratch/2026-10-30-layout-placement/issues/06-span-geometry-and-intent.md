# 06: Point-to-point span geometry and Server intent

**What to build:** a pure function that expands two opposite corner cells, a
shape and a gap into the ordered cells they span, and a Server intent that
takes the two corners and lays that span through the same per-cell judging,
tick budget and undo group as a directional layout. The preview answer covers
a span too, with an unset B taken from the aim.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

Spec: [layout placement](../spec.md), amendment 2026-10-30.

- [ ] A line spans the longest axis from A towards B, a rectangle the two
      longest and flat at A's level on the third, a box all three; ties prefer
      X, then Z, then up.
- [ ] The gap steps from A towards B; a span over 256 Voxels is refused.
- [ ] The Server judges every cell for reach, sight and occupancy; corners
      grant nothing, and out-of-reach or blocked cells are skipped.
- [ ] One undo reverses the whole span, atomically as before.
- [ ] Directional layouts behave exactly as before.
