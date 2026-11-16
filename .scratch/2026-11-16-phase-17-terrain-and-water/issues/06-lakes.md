# 06: Region drainage and lakes (17b)

Status: ready-for-agent

Blocked by: 04

Spec: [Phase 17](../spec.md).

## What to build

Regions with a jittered lattice hashed from the seed, a fixed halo and a bounded LRU cache; priority-flood basins with their spill heights feed `water_surface_m`.

## Acceptance criteria

- [ ] Any Chunk order, any cache state, same result.
- [ ] Every basin has a spill; lakes are flat at it.
- [ ] Region borders and the halo cut agree from both sides.
- [ ] The halo and region size are measured and recorded in the ticket.
