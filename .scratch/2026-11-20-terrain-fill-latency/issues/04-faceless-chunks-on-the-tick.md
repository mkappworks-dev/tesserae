# 04: A Chunk that cannot have a face is sent its empty mesh on the tick

Status: done

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

A level-0 Chunk that cannot have a face whatever the light (the cases `light_does_not_revive_either_declined_case` covers) is sent its empty mesh on the tick it lands, without a mesh job or the settled-light wait.

## Acceptance criteria

- [x] A streaming test: such a Chunk's empty mesh lands on its tick, and every other Chunk still waits for settled light.
- [x] The holes instrument reports no more fill or rule holes than 01's figures: an empty mesh sent early marks its Cell drawn and may discard a coarser level's ground poking into it before the Chunk below arrives.
- [x] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.

## Comments

- Built: `send_faceless` in `server/streaming.rs` runs each tick after the landings and sends an empty mesh, with no job and no settled-light wait, to every dirty Chunk of the mesh extent that `faceless` accepts; `emits_no_faces` is public in `tesserae-core` for it.
- Which cases go on the tick: a uniform Voxel walled in by six held neighbours that hide it (as `emits_no_faces` decides), and uniform air whose six held neighbours are uniform air, with no fluid among the seven. Both go on the tick the last of the seven lands. Air beside anything else keeps today's path.
- Why air beside ground waits: an empty mesh marks its Cell drawn (ADR-0019). Level 1 is the aligned downsample of level 0, so it never has ground in an air Cell, but levels 2 and up are made at their own Voxel size and stand up to a coarse Voxel above level 0's ground; one retained inside level 0's ring (a fast flight past level 1) would lose the top of its ground in the air Cell before the Cell below is drawn. The holes instrument cannot see that case: both Cells count as drawn.
- Tests: `a_faceless_chunk_is_sent_its_empty_mesh_on_the_tick` (open sky and deep rock, every relight and mesh job held: each such Chunk is sent empty on the tick its seventh Chunk lands, and nothing else is sent) and `air_beside_ground_waits_for_settled_light` (at the surface on land).

### Figures, 2026-11-20

The mesh table in [ticket 02](./02-empty-level-1-meshes-skip-light.md#figures-2026-11-20) and the lag table in [ticket 01](./01-lag-instrument.md#the-starting-figures-2026-11-20) cover this ticket. Level-0 mesh jobs fall 20% at 12 m/s, 45% at 64 m/s and 19% after a teleport, now that faceless Chunks skip the pool. Fill holes are as many as at ticket 01 (0 in `flight`, 7 to 11 frames in `sprint-flight` against 8, 96 to 112 after a teleport against 74 to 156), and no rule holes. Level-0 mesh CPU per second is 11% to 13% higher all the same, which removing jobs cannot cause; with ticket 02's level-1 figure it wants an A/B of mesh cost before the end figures. The teleport's level 0 settles in 2.02 to 2.17 s against 2.08 to 2.14 s.
