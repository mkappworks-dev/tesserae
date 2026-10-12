# Mesher: connected shapes in pass 2, and the hole-free test

Status: ready-for-agent
Blocked by: 04

Spec decision 6 and the hole-free section. Pass 1 is not edited.

## What to build

- `mesh_shaped`: for a connected Voxel, gather `around` from the
  `Neighborhood` with six `at` reads, call `connections`, emit `quads(voxel,
c)`. Non-connected Voxels take today's path with `Connections::NONE`.
- Boundary-quad culling asks `coverage_against(across, face ^ 1, voxel)` for a
  connected `across`, `coverage` otherwise. No call touches pass 1.
- `tests/meshing.rs`: a test-local `ShapeTable` registers a fence and a six-way
  pipe; `random_scene` places them among cubes, slabs, stairs and posts.
  The oracle computes connections **itself** from its own copy of the defs and
  the scene, never from `ShapeTable::connections`.
- Scenes that cross a Chunk boundary through `Neighborhood::new` with a real
  neighbour, so an arm toward the next Chunk is read from the shell.

## Tests

- The existing no-hole / no-double-draw property over all seeds, now with
  connected Shapes in the scene mix. Run it against a deliberately wrong
  predicate (an arm toward every neighbour; none; the wrong face) and against
  an over-eager `coverage_against`, and record that each is caught - the step 5
  tickets' mutation habit.
- A line of fences draws no hidden end caps; a fence beside a stair, a slab and
  a cube draws its arm exactly where the table says.
- Cross-check: for a random scene, the arms in the mesh, in `collision` and in
  `selection` agree voxel by voxel. (Ticket 06 supplies the sweep and ray sides
  and extends this; this ticket checks mesh against table.)
- `Neighborhood::isolated` treats the outside as air: a fence at the scene edge
  connects to nothing outside it.

## Done when

`./scripts/cargo-ci` passes; `greedy_mesh/terrain`, `/solid`, `/checkerboard`,
`terrain_stairs_at_cap` and `all_stairs` show no movement against the baseline,
and the PR reports the numbers.
