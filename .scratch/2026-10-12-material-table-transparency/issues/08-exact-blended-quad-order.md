# Blended quads in an exact order, re-sorted when it can change

Status: done
Blocked by: 04

Ticket 04's sort draws a Chunk's blended quads farthest centre first and
re-sorts after the Eye moves a Voxel or a sixteenth of its distance. Neither is
exact: two long quads can overlap with the nearer one's centre farther away,
and an order can be stale within the threshold.

## What to build

- Order quads by separating planes. Every quad is axis-aligned, so for two of
  them one's plane usually has the other wholly on one side; the one on the
  Eye's side of that plane is in front. Parallel quads: the farther plane is
  behind. A topological order of that relation, farthest centre first among
  quads it does not order and when it has a cycle (two greedy quads that cross,
  which only a checkerboard of two Materials makes).
- Re-sort exactly when the order can change: the relation depends only on which
  side of each quad's plane the Eye is, so a mesh re-sorts when the Eye crosses
  one of its planes and never otherwise. Replaces both thresholds.
- Keep the flat-layer skip.

## Acceptance criteria

- A ray-cast test: for the gallery's glass house and pool, and for a pair of
  long quads that centre order gets wrong, from views all around, every ray
  meets its blended layers in back-to-front draw order.
- Moving inside the same cell of the planes never re-indexes; crossing a plane
  that changes the order does.
- Sort count and worst frame on a flooded flight no worse than ticket 06's
  figures.
- `./scripts/cargo-ci` passes.

## Comments

Built, with two changes to the plan and one limit.

- **The relation is wider than the quads' own planes.** Two quads are ordered
  by any axis along which their boxes are disjoint, not only by one's own
  plane: the gallery's water and the glass house's side wall overlap on screen
  and neither's plane separates them, only the gap along z between them. The
  ray-cast test caught it. So the re-sort trigger watches every box bound on
  each axis, not only the planes.
- **Pairs are pruned per octant around the Eye.** Inside one octant every point
  is in front of its diagonal, so each quad's box clipped to the octant
  projects onto a plane across it; quads whose projections do not overlap in
  any octant share no ray. Conservative, so exactness holds, and it works for
  the mesh the Eye is inside.
- **A frame budget**, nearest mesh first, 200 us. One that is not reached keeps
  its order a frame longer.
- **Limit: meshes over 256 quads keep the farthest-centre order.** On a World
  flooded to 44 m that is 23 of 398 non-flat blended meshes, all lake shores. A
  far shore seen edge-on puts nearly every pair on screen together, and its sort
  was 0.5 to 0.8 ms at 376 quads.

Tests: a ray-cast test over three scenes (the gallery's glass house and pool, 24
views; two panes, 26; a strip of water over a glass cube near one end, 25),
every ray's blended layers drawn back to front; a test that the strip defeats
the farthest-centre order (so the fixture means something); moving within one
cell of the bounds never re-indexes. Swapping the exact order for the centre
one fails the ray-cast test. The cell check is a cost saving and not
observable: inside a cell the recomputed order is the same.

Cost, a 6 s flight over a World flooded to 44 m, load 5 to 9: about 710 sorts
(1130 with ticket 04's thresholds) and a worst sorting frame of 0.40 to 0.51 ms
(0.22 to 0.48), one run at 1.9 ms whose Server tick also spiked to 3.6 ms. Dry,
about 44 sorts and 98 to 158 us.

