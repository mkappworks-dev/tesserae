# 07: A Chunk above everything that can stand over its columns returns air, if it is worth it

Status: wontfix

Blocked by: 05

Spec: [Generation cost](../spec.md).

## What to build

First the gate: the share of level-0 generation in Chunks whose floor is above every column's top, from the sampled flight after 05. At 10% or less, closed `wontfix` with the figure. Otherwise a Chunk whose floor is above `surface_top_m` over its footprint widened by a tree's reach, with no Structure claim near it reaching its floor, returns air without the column pass, the Plant pass or `fields_near`. Neighbour Cells' column caches are not built for it.

## Acceptance criteria

- [x] The gate's figure in Comments.
- [ ] ~~If built: over a sweep of columns at 1.0 and 0.5 m, every Chunk the early-out takes is air in the full pass, over trees, crop fields, Water and Structures.~~
- [ ] ~~If built: the fingerprint and the committed digests match; bench figures in Comments.~~

## Comments

### The gate, 2026-11-20

Chunks whose floor stands more than 10 m (the overhang's reach) above every column's top over them are 1.9% to 2.7% of generation, under the 10% gate: closed. Measured with an uncommitted harness at fa6fd574: from a cold generator, the 3×3 Chunk columns round the first column along the bench's scan with ground 4 m above the sea, every Chunk from 96 m under the ground to 96 m over it (a ring's vertical reach), column by column, bottom to top, thread CPU time per Chunk summed over five generators:

| World | 1 m | 0.5 m |
| --- | --- | --- |
| The bench's | 2.4% of 30.5 ms | 2.7% of 39.0 ms |
| Its crop-field twin | 2.1% of 33.9 ms | 1.9% of 57.2 ms |

Such a Chunk already skips the column pass's work below its floor; what it still pays (the Plant pass and the crop fields) is shared through the caches with the Chunks under it.
