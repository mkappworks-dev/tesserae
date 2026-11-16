# 26: A Door's leaf opens as one panel against its jamb

Status: ready-for-agent

Blocked by: 22

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found a double Door's two leaves opening "in the same side". The
hinge data are right: ticket 22's leaves hinge on their outer edges, and the
World stores them mirrored (checked in the play-tested World: the low leaf
opened from orientation 7 to 4, the high leaf from 3 to 0). What a Player sees
is the fault. Use turns each Voxel of a Door about its own hinge, so a 2 m
leaf opens as two 1 m slats, one at its jamb and one in the middle of the
opening, and a pair reads as four narrow doors, two per side, each pair
swinging the same way.

Decided 2026-11-16 by the user: an open leaf shows a single panel against its
hinge jamb, and its other cells show nothing.

- An open Door Voxel is **folded** when the Voxel on its hinge side, the face
  its open panel lies on, is an open Voxel of the same Door turned the same
  way. Only the cells on a leaf's hinge edge stay drawn.
- A folded Voxel draws nothing and the aim ray passes it, so a Player sees and
  aims through the open doorway. Bodies already pass an open Door wider than
  one Voxel.
- The rule is local and stores nothing: a World saved with open Doors shows
  them folded when reopened.
- Closing is unchanged: Use on the panel toggles the whole leaf back.

## Acceptance criteria

- [ ] An open single and double Door mesh only their hinge-edge Voxels, at
      1.0, 0.5 and 0.25 m, tested in `tesserae-core`'s mesher.
- [ ] The aim ray passes a folded Voxel and strikes what stands behind it, and
      still strikes the panel at the jamb.
- [ ] A closed Door and a one-Voxel Door draw as before.
- [ ] Opening and closing a Template's double Door through the Server's Use
      leaves the pair mirrored.

## Comments
