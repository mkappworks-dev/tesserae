# 10: Drag to pick, handles and prompts for point to point

**What to build:** in point to point, pressing Place sets A and holding it
previews the span to the aim, releasing sets B, and Place then commits. A and
B are drawn in the world as markers with the span's size in Voxels and metres,
and the card names the keys for what they do at each step.

**Blocked by:** None (can start immediately)

**Status:** done

Spec: [layout placement](../spec.md), amendment 2026-10-30 (4). The user
confirmed the reading of "like Lay of the Land"; its tools were not visible to
us, so the drag, the handles and the prompts are our design.

- [x] A held Place previews the span to the aim; releasing sets B; a press that
      never moves picks A only.
- [x] Click, click, click still works, with the same cap and reach rules.
- [x] A and B are drawn as markers on their cells, with the span's size in
      Voxels and in metres beside them.
- [x] The card's key prompts follow the current bindings and the current step.
- [x] Menu, focus and device changes drop the points and never commit.
- [x] Shapes are unchanged; nothing leaves the Voxel lattice.

## Comments

### 2026-10-30: implemented

A press on A marks a drag; letting go on another cell sets B, letting go where
it began keeps A alone, and Remove, a menu or a lost focus ends the drag. B
still goes through the same cap and reach checks. The markers are outlined
cells, amber once picked and white while the aim stands in for B, with a text
label projected beside each. The size shows span extent in Voxels and metres
and does not count the gap. The markers and labels were not seen on screen,
only their logic is tested.
