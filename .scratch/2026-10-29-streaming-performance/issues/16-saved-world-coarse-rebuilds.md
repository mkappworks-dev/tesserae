# 16: Rebuild a saved World's coarse levels only where they changed

Status: ready-for-agent
Blocked by: none

Spec: [streaming performance](../spec.md).
PR boundary: on `tech/streaming-coarse-cache`, before 05's decision.

## Why

05's smoke runs found coarse generation on a Persistent World 7 to 25 times
dearer per Chunk than on an Ephemeral one at levels 2 to 5, with about a third
more jobs than the load extent. The demo's fluids give level-0 Chunks saved
revisions within seconds, and `server/lod.rs` then does two things:

- On every level-0 cover change, a saved World marks the covering Chunk stale
  at every coarse level; an unsaved World marks level 1 only. ADR-0011's
  Persistent Worlds section asks for this on purpose: saved changes are
  durable, so they must reach every level, and "invalidation follows all
  affected ancestors". It marks them all at once, though, whether or not the
  level below came out different.
- A coarse Chunk over a branch with saved changes is built by
  `generate_persistent`, which derives every relevant finer level again, down
  to level 1, inside the one job, even where that level's Chunk is held.

## What to build

1. On a saved World a level-0 change marks only the lowest coarse level holding
   (or generating) its cover, which is level 1 where level 0 is held. A higher
   level is marked when the Chunk below it lands as a replacement that differs
   from the one it replaced, and again only its lowest held cover.
2. A coarse job over a branch with saved changes reads the finer level's held
   coarse Chunk for that branch when it is current (held, not stale, not being
   regenerated), and derives it as today only where it is not.
3. ADR-0011's Persistent Worlds section says so in a sentence.

## Acceptance criteria

- [ ] On a saved World, a level-0 change marks the level-1 cover stale and no
      higher level.
- [ ] A level-1 replacement identical to the Chunk it replaced marks nothing
      above it; one that differs marks the level-2 cover.
- [ ] A rebuild that reads held finer coarse Chunks makes the same Chunk,
      overlay and seam openings as `generate_persistent`, asserted in a test.
- [ ] Unsaved Worlds unchanged: the existing LOD tests pass.
- [ ] `restart` and `revisit` smoke runs, against 05's `bd97491` numbers:
      per-job coarse generation ms at levels 2 to 5, the backlog at +10 s,
      level-0 settle and coarse-caught-up times. Recorded here, and whether
      05's cache is still needed answered in 05's Comments.

## Comments
