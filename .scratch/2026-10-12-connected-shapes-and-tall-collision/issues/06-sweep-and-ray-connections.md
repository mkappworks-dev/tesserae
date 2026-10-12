# Sweep and ray see the arms

Status: ready-for-agent
Blocked by: 04

Spec decisions 3, 8 and 9 for the other two callers of `connections`.

## What to build

- In `collide`, when the Voxel at a visited cell `is_connected`, read its six
  face-neighbours through `VoxelVolume::voxel` (`None` has no group and seals
  nothing, as in the mesher), call `connections`, and use
  `collision(voxel, c)` for the sweep and `selection(voxel, c)` for the ray.
  Non-connected Voxels never read neighbours.
- Extend the cross-check from 05 to the sweep and the ray: for a random scene,
  the arms a body is stopped by, the arms a ray strikes and the arms the mesh
  draws agree.

## Tests

- A body walks along a fence line and is stopped by the arms, by the posts, and
  not by the gap between two unconnected posts of a different group.
- Walking into a fence at y = 1.0 is blocked by its 1.5-Voxel collision and a
  ray at the same height strikes the **arm**, not the air above it.
- A fence beside an unloaded cell (`None`) does not connect to it, and the
  unloaded cell is still solid to the sweep.
- Same scenes at Voxel size 1.0 and 0.25 agree in metres.

## Done when

`./scripts/cargo-ci` passes; the sweep and ray benches from 01 on cubes and
slabs show no movement beyond noise (the connected path is a branch away from
them), and the PR reports the numbers.
