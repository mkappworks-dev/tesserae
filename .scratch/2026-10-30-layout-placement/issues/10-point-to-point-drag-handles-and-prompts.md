# 10: Drag to pick, handles and prompts for point to point

**What to build:** in point to point, pressing Place sets A and holding it
previews the span to the aim, releasing sets B, and Place then commits. A and
B are drawn in the world as markers with the span's size in Voxels and metres,
and the card names the keys for what they do at each step.

**Blocked by:** None (can start immediately)

**Status:** needs-info

Spec: [layout placement](../spec.md), amendment 2026-10-30 (4). Waiting on the
user to confirm the reading of "like Lay of the Land": its tools were not
visible to us, so the drag, the handles and the prompts are our design.

- [ ] A held Place previews the span to the aim; releasing sets B; a press that
      never moves picks A only.
- [ ] Click, click, click still works, with the same cap and reach rules.
- [ ] A and B are drawn as markers on their cells, with the span's size in
      Voxels and in metres beside them.
- [ ] The card's key prompts follow the current bindings and the current step.
- [ ] Menu, focus and device changes drop the points and never commit.
- [ ] Shapes are unchanged; nothing leaves the Voxel lattice.
