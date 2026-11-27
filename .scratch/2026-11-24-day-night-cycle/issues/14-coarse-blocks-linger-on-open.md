# 14: Coarse blocks linger after a saved World opens

Status: done

Spec: [Day and night cycle](../spec.md).

## What to build

Opening the user's saved World, the view past level 1 shows big coarse-level
blocks for 10 to 15 s before the finer levels replace them. On a fresh World
from the same seed both `main` and this branch had filled in at 4 s, so the
branch does not slow streaming there; the saved World's load path is the
suspect. Find why and fix it, or record that it is expected and why.

Reproduce with ticket 13's World copy and spawn, at
`TESSERAE_SCREENSHOT_SETTLE_S=4` against `30`. `scripts/profile-frames`
fixtures and their fill metrics may measure it better than captures.

## Acceptance

- [x] The cause found and written here.
- [x] Fixed, with the time to fill measured before and after, or closed as
      expected with the reason.

2026-11-27, cause, from `sample` profiles of the copy opening: the World holds
1341 saved Chunks. Nobody edited them: plant growth (vines, fruit, crops)
changes a Chunk within seconds of it loading, so every Chunk with a growing
plant near the Player's path is saved (a fresh World left alone for 30 s saved
143). Two costs came of that:

- Opening decoded every saved Chunk about five times: `Commit::compatible`
  restored each to validate it, and it runs in `Writer::acquire` (twice),
  `inspect`, `prepare` and `load_world` before `install` restores them for
  real. Setting every Voxel was nearly all of it; one pass ran on the main
  thread as an 0.75 to 1 s frame.
- A coarse Chunk over a saved one takes `generate_persistent`, which derives
  the whole branch under it, bridges and all, unless the finer level already
  shows it. On opening none does, so each branch was derived again by every
  coarser level's job: 94% of the LOD jobs' CPU in the first seconds.

Fix: `SavedChunk::validate` makes every check `restore` makes without building
the Chunk, and `compatible` uses it. A Session memo (`Derived`) keeps each
derived coarse Chunk with the saved Chunks it read and serves it while those
are the same handles, so a branch is derived once; an edit saves a new handle
and misses it. Bounded at 4096 Chunks, in memory only.

Measured on the copy at the ticket's spawn, 30 s settle, `dev` release, M5,
load 4 to 5; seconds from the window, two runs each:

| Saved World              | Before    | Validate  | Validate + memo | Fresh World |
| ------------------------ | --------- | --------- | --------------- | ----------- |
| Player spawned           | 3.2–3.3   | 1.4–1.6   | 1.3             | 0.5         |
| Spawn held and lit       | 7.7–8.6   | 6.7–7.3   | 4.9             | 1.4         |
| Level 0 shown            | 10.5–11.5 | 9.5–9.8   | 7.2–7.6         | 3.4         |
| Level 5 shown            | 12.4–13.4 | 11.6–12.3 | 9.0–9.3         | 4.9         |
| Levels 1 to 4 filled     | 39.6–41.4 | over 30   | 28.3–29.4       | 29.9        |
| Worst frame while filling| 735–982 ms| 55–60 ms  | 61–66 ms        | 31 ms       |

"Fresh World" is the copy's `world.json` alone: the same seed, no saved
Chunks. Levels 1 to 4 now fill as fast as on a fresh World; their 30 s there
is the known coarse-level load time (`tech/coarse-level-load-time`). Level 0
still shows about 4 s later than on a fresh World: the first derivation of
each saved branch, mostly level 1's bridge searches, shares the three async
threads with level 0's own Chunks.
