# 11: Felling

Status: ready-for-agent

Blocked by: 06, 10

Spec: [Phase 18](../spec.md).

## What to build

Cutting a trunk breaks what depended on it. When a trunk Voxel of a `tree` Plant
changes, the Server wakes its neighbours and evaluates the Felling region: the
grown Voxels of that tree connected through faces that no longer reach a
grounded grown trunk (one standing on a Voxel that is not grown), bounded by the
Plant's declared size in metres. Only grown Materials take part, so what a
player placed is never felled, whatever it looks like. A region over the size
bound does not break. It breaks top-down over successive Logical
steps under `FELLED_PER_STEP`, each Voxel dropping its Item like any removal. The
cascade is not an Edit the journal records; undo restores the cut Voxel only.
Falling as a body, in a direction, is step 24's.

## Acceptance criteria

- [ ] Cutting a grown tree's trunk breaks the part above the cut over several
      steps, top-down, never more than `FELLED_PER_STEP` per step, and drops
      Items.
- [ ] A felled tree's fruit Voxels drop their Stage's yield; unripe fruit drops
      only what its yield declares.
- [ ] Every Felling drops at least one sapling, once for the whole tree and not
      per Voxel, picked up and planted to grow another tree; no sapling drops
      when the cut leaves the tree grounded.
- [ ] A cabin of placed logs and leaves, and a placed leaf touching a grown
      trunk, are never felled; a region over the size bound is not either.
- [ ] Cutting a trunk Voxel that leaves the tree grounded breaks nothing else.
- [ ] Burn-out and a Mod Command's removal fell by the same rule.
- [ ] Undo after a felling restores the Voxel the Player cut and no more; the
      journal's group count is one.
