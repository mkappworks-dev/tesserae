# 02: An empty level-1 mesh skips the coarse light

Status: done

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

The mesher's test that a Chunk emits no faces whatever its light is public in `tesserae-core`, and `mesh_coarse` asks it first, returning the empty mesh without computing `coarse_light_in`.

## Acceptance criteria

- [x] `mesh_coarse` of a Chunk that emits no faces equals today's result.
- [x] Level-1 mesh CPU and its share of empty results, before and after, in Comments.
- [x] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.

## Comments

Built: `tesserae_core::emits_no_faces` is public and takes the centre's overlay and its neighbours' Chunks but no light, so fluid in the overlay is part of the answer rather than a second check each caller must remember. `mesh_with` asks it as before, and `mesh_coarse` asks it first and returns the empty mesh without calling `coarse_light_in`. `mesh_coarse` takes its generator through a small `Source` trait (`Surface` plus the water) so a test can hand it one that fails when asked where the ground is.

Proved by `a_chunk_is_faceless_in_exactly_the_two_declined_cases` (core `meshing`), `a_coarse_chunk_with_no_faces_is_meshed_empty_without_its_light`, and its witness `a_coarse_chunk_with_faces_asks_for_its_light` (`server::lod`), which shows the probe generator does fail when a Chunk is lit.

For the figures: `frames.flow.csv` has a `mesh_empty` column (mesh jobs above level 0 whose mesh was empty, since the World opened) and `streaming.json`'s `workers` reports `mesh_empty` and `mesh_empty_share` per coarse level. Which results are empty does not change with this ticket, so the share from the new build is also the share before it; only the mesh CPU moves.

### Figures, 2026-11-20

fa6fd574 holds this ticket, ticket 04 and the sister spec's ticket 05; fded14c8 is ticket 01. Built and run as in [the sister spec's ticket 05](../../2026-11-20-generation-cost/issues/05-small-caches.md#the-flight-after-05-2026-11-20), two rounds of fa6fd574 and three of fded14c8:

| Fixture | Build | L0 mesh jobs | L0 mesh ms/s | L1 mesh jobs | L1 mesh ms/s | L1 empty share |
| --- | --- | --- | --- | --- | --- | --- |
| `flight` | fded14c8 | 659 | 27.6 | 4576 | 277.6 | – |
| `flight` | fa6fd574 | 526 | 30.7 | 4602 | 302.6 | 16% |
| `sprint-flight` | fded14c8 | 1807 | 39.7 | 4011 | 184.4 | – |
| `sprint-flight` | fa6fd574 | 1002 | 44.4 | 4064 | 199.2 | 32% |
| `teleport` | fded14c8 | 899 | 33.2 | 2877 | 137.6 | – |
| `teleport` | fa6fd574 | 731 | 37.5 | 2903 | 146.2 | 28% |

A sixth to a third of level-1 mesh jobs are empty (16% at 12 m/s, 32% at 64 m/s), and three quarters at level 2. Skipping their light shows no gain: level-1 mesh CPU is 5% to 9% higher at the same job count. Nothing here makes a non-empty level-1 mesh dearer, so the empties' light was cheap and something else moved; the level-0 mesh figures in ticket 04 moved the same way, which needs its own A/B before the end figures.
