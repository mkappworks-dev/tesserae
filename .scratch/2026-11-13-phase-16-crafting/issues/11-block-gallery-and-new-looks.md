# 11: Block gallery and new looks

Status: done

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

- [x] One documented command regenerates the gallery's textures from Terra.
- [x] `/blocks` shows every Terra Material, its Shapes, and the Door closed and
      open at Player scale.
- [x] The new Workbench, Chest and Door looks are approved in the gallery by the
      user before their Terra commit.
- [x] Terra's texture fingerprint and appearance pins are updated.

## Comments

2026-11-14: the gallery and the new Workbench, Chest and Door looks are on
`feat/phase-16-10-11-wood-and-looks`, the looks alone in its last commit. They
await the user's approval in `/blocks` before merging. A texture covers one
Voxel, so a Door's handle repeats on every cell: twice at 1 m, 32 times at
0.25 m; the gallery's Voxel size control shows it.

2026-11-14: the user picked W4 for the Workbench and C6 for the Chest. The
Workbench is a birch top with a saw and hammer over walnut sides with pale legs,
always placed as `terra:table`: four legs under a thick top, open beneath, which
collides as a whole cell so nothing snags between the legs. The Chest stays a
cube of framed red-brown planks with a rope-loop handle; Terra has one side
texture, so the handle shows on all four sides. The gallery draws each Material
in the Shape it places in. A Shape a `voxel-uses` row fixes (`terra:panel`,
`terra:table`) is no longer offered as a Shape chip or by the Shape key,
superseding ticket 13's sixth chip.
