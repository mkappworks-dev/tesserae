# 31: Placed Structures sit level with the ground

Status: ready-for-agent

Blocked by: 08, 14

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found every placed Structure's doors one Voxel above the ground
outside.

- The place Tool's anchor is the cell in front of the face the ray strikes,
  and the Placement puts the Ground layer's lowest Voxel layer on it. Aimed at
  the ground's top face, that is the air above the ground, so the floor stands
  one Voxel up. Worldgen puts the Ground layer on the surface Voxel instead.
- At finer Voxels both are off: the Ground layer is a 1 m cell, so with its
  lowest Voxel layer on the surface Voxel its top stands up to 1 m less one
  Voxel above the ground, 0.5 m at 0.5 m and 0.75 m at 0.25 m.

Fix both so the Ground layer's top Voxel layer replaces the ground's top Voxel
layer: a floor and a door's sill are level with the ground outside at 1.0, 0.5
and 0.25 m, from the place Tool and from worldgen alike. A Template whose
cells are on whole metres still lands on whole metres.

## Acceptance criteria

- [ ] Placing a Template aimed at flat ground puts its floor's top level with
      the ground's top at 1.0, 0.5 and 0.25 m, tested through the Server.
- [ ] A worldgen Structure and a Settlement building stand with their floor's
      top level with the ground beside them at every size, tested.
- [ ] The ghost still shows what then lands.
- [ ] Worlds without Structures stay byte-identical.

## Comments
