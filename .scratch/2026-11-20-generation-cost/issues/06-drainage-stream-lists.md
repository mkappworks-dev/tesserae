# 06: `Drainage::wet` walks only the streams in reach, if it is still worth it

Status: wontfix

Blocked by: 05

Spec: [Generation cost](../spec.md).

## What to build

First the gate: `Drainage::wet`'s share of level-0 generation in a sampled flight after 05. At 15% or less, closed `wontfix` with the figure. Otherwise a drainage region keeps, per lattice cell, the stream segments its stencil reaches, in the order the scan meets them, and `wet` walks that list, skipping the meander noise where it is empty.

## Acceptance criteria

- [x] The gate's figure in Comments.
- [ ] ~~If built: `wet` equals the scan bit for bit over many seeds and points, including points with no segment in reach and beside a region border.~~
- [ ] ~~If built: the fingerprint and the committed digests match; bench figures in Comments.~~

## Comments

### The gate, 2026-11-20

`Drainage::wet` is 12.8% of level 0's generation in the sampled flight of fa6fd574 (ticket 05's Comments), under the 15% gate: closed. At the coarser levels it is 2.3%.
