# Collision: a sweep and a ray, in metres

Status: done
Blocked by: 01

DESIGN.md: "Sweep an AABB against the voxel grid directly." The sweep has no
caller yet, and spec decision 9 explains why it lands anyway: it is the cheapest
honest test of the collision half of the Shape table, and the ray half is the
edit path's picking. Neither takes a dependency.

## What to build

`tesserae-core/src/collide.rs`:

- `trait VoxelVolume { fn voxel(&self, at: [i32; 3]) -> Option<Voxel>; }`.
- `Aabb { min: [f32; 3], max: [f32; 3] }` in metres.
- `sweep(vol, shapes, voxel_size, aabb, delta) -> Sweep`: resolve x, y, z in
  turn against the rotated collision boxes of every Voxel the box can reach;
  return the movement actually made and which axes were blocked. `None` from the
  volume is solid, so nothing leaves the loaded world.
- `raycast(vol, shapes, voxel_size, origin, dir, reach_m) -> Option<RayHit>`: a
  cell DDA that, in each cell, intersects the ray with that cell's collision
  boxes, so it enters a stair's notch and stops at the tread. `RayHit { at:
  [i32; 3], face: usize, distance_m: f32 }`.
- Every position and distance is in metres and `voxel_size` is the only
  conversion. No step height, no gravity: those are a controller's.

## Tests

- Standing on a bottom slab rests at 0.5 m, and on a top slab at 1.0 m; walking
  into a top slab's underside from below is blocked at 0.5 m of headroom.
- A stair stops a walker at the first riser and lets it onto the tread.
- A box slides along a wall: blocked on one axis, free on the others.
- **The same physical scene at Voxel size 1.0 and 0.25 gives the same result in
  metres** (ADR-0003).
- A thin shape (the post) is passed by a box that does not overlap it and
  stopped by one that does.
- `collision: Some(vec![])` is walked through.
- A ray enters a stair's notch; the same ray against a cube stops at the cell;
  reach is honoured; a ray starting inside a cell works; the reported face is
  the one entered.
- A `None` region stops a sweep.

## Comments

Four mutations were applied and all were caught: collision reduced to "is it
solid?" (7 tests fail, which is the point of the step), the ray testing whole
cells instead of boxes (the notch test), `blocked` never reported (7 tests), and
the struck face reversed (2 tests).

A body that already overlaps a box is not pushed out and is not stopped by it;
it can move out. A ray that starts inside a box strikes it at distance zero.
Both are documented on the functions and tested for the ray.
