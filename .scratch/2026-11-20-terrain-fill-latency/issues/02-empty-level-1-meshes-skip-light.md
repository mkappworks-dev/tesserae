# 02: An empty level-1 mesh skips the coarse light

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

The mesher's test that a Chunk emits no faces whatever its light is public in `tesserae-core`, and `mesh_coarse` asks it first, returning the empty mesh without computing `coarse_light_in`.

## Acceptance criteria

- [ ] `mesh_coarse` of a Chunk that emits no faces equals today's result.
- [ ] Level-1 mesh CPU and its share of empty results, before and after, in Comments.
- [ ] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.
