# Exact blended order for large meshes

Status: done
Blocked by: 08

Ticket 08 keeps meshes over 256 quads on the farthest-centre order, because a
far shore seen edge-on puts nearly every pair of its quads on screen together
and the pairwise order cost 0.5 to 0.8 ms at 376 quads. Most of those pairs are
two quads in one plane facing one way, which can never overlap and which the
relation already refuses to order.

## What to build

- Skip coplanar same-facing pairs when the candidates are built, before the
  relation is asked, and measure what that leaves.
- Raise the cap as far as the measured cost allows within the frame budget.

## Acceptance criteria

- The ray-cast test passes, and a test with a mesh over the old cap is drawn
  back to front.
- On a flooded flight, the worst sorting frame stays within ticket 08's figures
  and the share of meshes over the cap is reported.
- `./scripts/cargo-ci` passes.

## Comments

Built. Coplanar pairs are dropped while the candidates are swept, and the cap
is 2048 quads, up from 256.

- The first test of a large mesh, a 3D checkerboard of glass and water, passed
  with the old cap: its quads are all a Voxel square, and the farthest-centre
  order is right for those. It now holds the strip of water over a glass cube
  too, which that order gets wrong (36 rays from above the cube's end), and it
  fails at the old cap and passes at the new.
- Cost, four 6 s flights over a default World with its lakes, load 4.8 to 5.9:
  worst sorting frame 238 to 544 us, one at 1.44 ms (ticket 08's: 400 to 510,
  one at 1.9), about 460 sorts. The largest blended mesh seen on a flooded
  World is 505 quads, so none reaches the cap.
