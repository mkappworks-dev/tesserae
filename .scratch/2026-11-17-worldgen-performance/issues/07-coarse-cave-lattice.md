# 07: A coarse lattice for the cavern and tunnel fields, if the density is still worth it

Status: ready-for-agent

Blocked by: 06

Spec: [Worldgen performance](../spec.md).

## What to build

First a re-measure: the 3D density's share of generation once ticket 06 is settled. At 10% or less, this ticket is closed as `wontfix` with the figure. Otherwise the cavern field and the two tunnel fields are sampled on a 4 m lattice anchored at the World origin in metres and interpolated between samples, each field before it is thresholded, so it holds at every Voxel size; the range bounds that rule boxes out widen by a few ulps. The overhang push stays exact per Voxel. Cave shapes would move, and version 3 is frozen, so a passing gate does not build it here: the ticket goes to `ready-for-human` with the figure, for a decision on a new Generator version. Whichever way the gate goes, the last flight profile is recorded here.

## Acceptance criteria

- [ ] The re-measured share and the final flight profile, against the ticket 01 one, are in Comments.
- [ ] If built: generated water has no air below or beside it, flooded caverns stay sealed from tunnels, and no overhang reaches water or a Structure, at 1.0 and 0.5 m.
- [ ] If built: the same metre point is carved at every Voxel size, and a box the bound rules out holds no carve.
- [ ] If built (only under a new Generator version): only cave Voxels differ from the ticket 01 fingerprint.
