# 26: A Door's leaf opens as one panel against its jamb

Status: done

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

- [x] An open single and double Door mesh only their hinge-edge Voxels, at
      1.0, 0.5 and 0.25 m, tested in `tesserae-core`'s mesher.
- [x] The aim ray passes a folded Voxel and strikes what stands behind it, and
      still strikes the panel at the jamb.
- [x] A closed Door and a one-Voxel Door draw as before.
- [x] Opening a Template's double Door leaves the pair mirrored, each leaf
      onto its outer edge, at 1.0, 0.5 and 0.25 m (checked on Terra's inn).

## Comments

Where it lives: `VoxelUses::folded` (`usable.rs`), asked by `mesh_shaped`
(`mesh.rs`) for every shaped Voxel before it counts or draws, and by
`raycast_until` (`collide.rs`), so the Server's aim, Use, Remove and Place all
pass a folded Voxel. The check scans the Door rows without allocating; a
Voxel that is not an open Door with a fixed Shape answers at once.

Tests: `an_open_leaf_draws_only_its_hinge_edge_and_the_aim_ray_passes_the_rest`
(2, 4 and 8 Voxel leaves, hinged either way: 3 Voxels drawn of each open leaf,
all of a closed one; the ray strikes the jamb panel and passes the rest to the
stone behind); `a_player_walks_through_each_leaf...` in
`demo_games/rooms.rs` now asks each leaf of the inn's pair to open onto its
outer edge. `breaking_any_cell_removes_the_whole_door...` aims its removal of
an open Door at the jamb panel, as the folded Voxels let the ray through.

Checked in play on the play-tested World's warehouse: the opened pair shows
one panel against each jamb, and the opening is clear.

Known gap: two single Doors side by side, hinged the same way and both open,
fold the second into the first, so the second shows no panel.
