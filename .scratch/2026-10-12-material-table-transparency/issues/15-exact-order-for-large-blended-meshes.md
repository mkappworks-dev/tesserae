# Exact blended order for large meshes

Status: ready-for-agent
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
