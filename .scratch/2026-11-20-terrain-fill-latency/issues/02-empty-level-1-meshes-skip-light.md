# 02: An empty level-1 mesh skips the coarse light

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

The mesher's test that a Chunk emits no faces whatever its light is public in `tesserae-core`, and `mesh_coarse` asks it first, returning the empty mesh without computing `coarse_light_in`.

## Acceptance criteria

- [x] `mesh_coarse` of a Chunk that emits no faces equals today's result.
- [ ] Level-1 mesh CPU and its share of empty results, before and after, in Comments.
- [ ] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.

## Comments

Built: `tesserae_core::emits_no_faces` is public and takes the centre's overlay and its neighbours' Chunks but no light, so fluid in the overlay is part of the answer rather than a second check each caller must remember. `mesh_with` asks it as before, and `mesh_coarse` asks it first and returns the empty mesh without calling `coarse_light_in`. `mesh_coarse` takes its generator through a small `Source` trait (`Surface` plus the water) so a test can hand it one that fails when asked where the ground is.

Proved by `a_chunk_is_faceless_in_exactly_the_two_declined_cases` (core `meshing`), `a_coarse_chunk_with_no_faces_is_meshed_empty_without_its_light`, and its witness `a_coarse_chunk_with_faces_asks_for_its_light` (`server::lod`), which shows the probe generator does fail when a Chunk is lit.

For the figures: `frames.flow.csv` has a `mesh_empty` column (mesh jobs above level 0 whose mesh was empty, since the World opened) and `streaming.json`'s `workers` reports `mesh_empty` and `mesh_empty_share` per coarse level. Which results are empty does not change with this ticket, so the share from the new build is also the share before it; only the mesh CPU moves.
