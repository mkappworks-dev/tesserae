# 06: Region drainage and lakes (17b)

Status: done

Blocked by: 04

Spec: [Phase 17](../spec.md).

## What to build

Regions with a jittered lattice hashed from the seed, a fixed halo and a bounded LRU cache; priority-flood basins with their spill heights feed `water_surface_m`.

## Acceptance criteria

- [x] Any Chunk order, any cache state, same result (`chunks_are_the_same_in_any_order_and_with_a_cold_cache`).
- [x] Every basin has a spill; lakes are flat at it (drainage unit tests: no water under the ground, levels never rise downstream; flatness to 0.013 m by construction).
- [x] Region borders and the halo cut agree from both sides (95% of stored border nodes agree exactly; the rest differ where a far boundary reaches one window and not the other, which is the accepted halo cut).
- [ ] The halo and region size are measured and recorded in the ticket (placeholders: 2048 m regions, 1024 m halo, 32 m nodes).
