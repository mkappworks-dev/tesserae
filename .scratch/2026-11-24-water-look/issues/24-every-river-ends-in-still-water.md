# 24: Every river ends in still water, and ponds

Status: done
Blocked by: 23

Spec: [Water that looks natural](../spec.md), Lakes. From the user's review:
rivers should end in a lake, a pond or the sea.

## What to build

A shallow basin a stream enters, 1 m deep or more, holds a pond instead of
being filled flat, so a stream ends in still water. Where a river crosses a
window's edge, the fine solve takes the coarse solve's catchment there, so it
has one width on both sides. Small ponds of 3 to 20 m, a hashed candidate per
128 m cell, stand at their rim's lowest point and need no lip; one also serves
as a spring pool.

## Acceptance criteria

- [ ] Every stream ends in a lake, a pond or the sea, or leaves what is
      stored; a test walks every stream.
- [ ] A river's width agrees on both sides of a region border; a test.
- [ ] Ponds on the map; a capture shown to the user.

## Comments

- 2026-11-27: one pond in a fifth of 128 m cells, 2.5 to 10 m in
  radius, where its rim is level within 1.5 m and 2 m above the sea, its
  water 0.4 m under the rim's lowest point. Not done: shallow basins a stream
  enters are not ponds, and the coarse catchment is not fed across the window
  edge. `/tmp/tesserae-water-look-captures/24-after/pond.png`. The crop-field fixture now asks for a field
  that runs into the next Chunk, as a pond moved the first one.
- 2026-11-27, the rest. A basin 1 m deep holding a lake, so a stream ends in a
  pond, flooded 7% or more of the valleys between mountains and dried a
  Settlement's bridge valley; reverted to 2.5 m. Every stream already ends in
  a lake, the sea or the edge of what is stored (the solve's own test).
  Feeding the coarse catchment in at a window's edge is not done: the edge
  nodes are the flood's outlets, so area put on them reaches nothing
  downstream; it needs inflow nodes at the edge, a change to the solve's
  seeding. The 1 km halo keeps widths close at a border; the user has the
  call on whether it is needed.
- 2026-11-27, window edges. Where the coarse solve's river crosses a window's
  edge inward, what it gathered outside (less what it took out of the window
  and brought back) enters on the lowest edge node near the crossing, and the
  edge four nodes either side is no outlet, so the river drains inward. On 8
  Terra worlds, border stream nodes a fifth or more apart in width: 5.3% to
  3.5% (worst 73% to 65%). The rest are valleys the 128 m lattice cannot see.
  A wider halo also helps, with inflow: 1.5 km 1.3%, 2 km 0.6%, at about 1.6x
  and 2.6x a region's solve; left at 1 km until the profile shows what the
  solve costs. Test `a_river_keeps_its_width_across_a_region_border`.
- 2026-11-27: the user kept the 1 km halo.
