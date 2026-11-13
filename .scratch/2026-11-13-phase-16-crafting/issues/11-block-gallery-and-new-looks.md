# 11: Block gallery and new looks

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Phase 16](../spec.md), amendment 2026-11-14.

## What to build

The mockup gains a `/blocks` page that previews every Terra Material as a
textured block, using Terra's real textures, so a look can be judged before it
ships. A dev export (an example or ignored test in the workspace that loads the
built Terra guest through the Mod loader) writes each Material's texture faces
as PNG plus a small JSON index into the mockup; the page draws each Material as
an isometric cube from those faces, shows the placeable Shapes, and shows the
Door closed and open beside a 1.8 m Player silhouette. Then the Workbench
(planked top with tool marks, legs on the sides), Chest (banded planks, a latch
on the front) and Door (closed: framed planked panel with a handle; open: the
panel swung flush to one edge, the rest clear) get new looks in Terra's
textures, exported to the gallery for the user's approval before they are
committed.

## Acceptance criteria

- [ ] One documented command regenerates the gallery's textures from Terra.
- [ ] `/blocks` shows every Terra Material, its Shapes, and the Door closed and
      open at Player scale.
- [ ] The new Workbench, Chest and Door looks are approved in the gallery by the
      user before their Terra commit.
- [ ] Terra's texture fingerprint and appearance pins are updated.
