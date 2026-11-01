# 16: Rebuild a saved World's coarse levels only where they changed

Status: done
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

- [x] On a saved World, a level-0 change marks the level-1 cover stale and no
      higher level.
- [x] A level-1 replacement identical to the Chunk it replaced marks nothing
      above it; one that differs marks the level-2 cover.
- [x] A rebuild that reads held finer coarse Chunks makes the same Chunk,
      overlay and seam openings as `generate_persistent`, asserted in a test.
- [x] Unsaved Worlds unchanged: the existing LOD tests pass.
- [x] `restart` and `revisit` smoke runs, against 05's `bd97491` numbers:
      per-job coarse generation ms at levels 2 to 5, the backlog at +10 s,
      level-0 settle and coarse-caught-up times. Recorded here, and whether
      05's cache is still needed answered in 05's Comments.

## Comments

### 2026-11-01: built, with smoke evidence

Why the paths differ: on purpose. ADR-0011 has an unsaved World's levels above
1 read the generator alone, so a change never needs to climb; a saved World's
changes are durable and must reach every level, so the save step marked every
ancestor at once. It still reaches every level, now one level at a time.

- `2ddbf35`: `mark_cover` marks the lowest level from 1 up that holds or is
  generating the cover; `land` marks the next level's cover when a replacement
  lands with different packing (`same_packing` on `Chunk` and `ChunkOverlay` in
  `tesserae-core`: equal packing proves equal Voxels, so a false "changed"
  costs one extra rebuild, never a missed one). Unsaved Worlds keep level 1
  only. Tests: `lod/tests/saved.rs`.
- `6782863`: `generate_persistent` reads held finer coarse Chunks that no mark
  or job calls out of date (`current_finer`), deriving only the rest.
  `a_rebuild_reading_held_finer_chunks_makes_what_deriving_them_again_makes`
  asserts equal Chunk, overlay and seam openings at levels 2 to 4 over a dry,
  a stone and a flooded saved Chunk; dropping the reused overlay fails it.
  ADR-0011's Persistent Worlds section says so.

Smoke runs, one each, `shot.sh`, release `profiling`: before is `bd97491`
(05's), after is `6782863`. Backlog is coarse generate waiting plus in flight
over levels 1 to 5, 10 s after the move.

| Run               | Coarse ms/job, levels 2-5, fill | Backlog at +10 s | Level-0 settle | Coarse caught up |
| ----------------- | ------------------------------- | ---------------- | -------------- | ---------------- |
| restart before    | 1.30, 2.62, 2.83, 4.56          | 2,876            | 0.76 s         | not by +10.9 s   |
| restart after     | 0.38, 0.36, 0.16, 0.16          | 2                | 0.77 s         | 7.95 s           |
| revisit before    | 1.43, 2.83, 3.00, 4.70          | 253              | 0.84 s         | not by +10.5 s   |
| revisit after     | 0.40, 0.39, 0.17, 0.18          | 2                | 0.76 s         | 4.61 s           |
| teleport, unsaved | 0.20, 0.19, 0.16, 0.18          | 0                | 0.81 s         | 4.91 s           |

Revisit's settle and catch-up are the return's; restart's settle follows a
0.27 s reopen with no frames, unchanged. Levels 2 and 3 still cost about
twice an unsaved World's per job, and levels 2 to 4 still run 1,780 to 2,230
jobs for a 1,639-Chunk extent by 21.5 s: the demo's fluids keep changing
level-0 Chunks, and those rebuilds now stop where nothing changed. Level-0
settle did not move; it never waited on coarse work.
