# 07: A Chunk above everything that can stand over its columns returns air, if it is worth it

Status: ready-for-agent

Blocked by: 05

Spec: [Generation cost](../spec.md).

## What to build

First the gate: the share of level-0 generation in Chunks whose floor is above every column's top, from the sampled flight after 05. At 10% or less, closed `wontfix` with the figure. Otherwise a Chunk whose floor is above `surface_top_m` over its footprint widened by a tree's reach, with no Structure claim near it reaching its floor, returns air without the column pass, the Plant pass or `fields_near`. Neighbour Cells' column caches are not built for it.

## Acceptance criteria

- [ ] The gate's figure in Comments.
- [ ] If built: over a sweep of columns at 1.0 and 0.5 m, every Chunk the early-out takes is air in the full pass, over trees, crop fields, Water and Structures.
- [ ] If built: the fingerprint and the committed digests match; bench figures in Comments.
