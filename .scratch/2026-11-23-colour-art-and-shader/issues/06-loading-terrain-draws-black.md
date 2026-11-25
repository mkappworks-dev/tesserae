# 06: Loading terrain draws black

Status: done

Spec: [Colour, art and light](../spec.md).

## What to build

A bug on `main`, found in this spec's captures: while far LOD levels are still
loading, their terrain draws black with orange dots before it fills in. Seen
at `jungle-top-medium` and `far` with a 30 s settle under host load; level 1
held 0 to 54 Chunks where a settled view holds about 5340.

Cause (diagnosed): a coarse Chunk was sent before a ground or water neighbour
at its level, and the dark beds and cave floors beneath showed through the gap.
Fix (user, Q61): a coarser level keeps drawing a region until every finer Chunk
covering it is sent. Holding a Chunk until every neighbour is sent was
rejected: it left large voids mid-load.

## Acceptance

- [ ] The cause named, with a reproduction (a test where the cause allows one).
- [ ] A capture taken mid-load at `jungle-top-medium` and `far` shows no black
      terrain.
- [ ] No regression in time to a settled view; `flight` GPU frame no worse than
      `main` beyond noise.

## Comments

2026-11-24: cause. Not missing Sky light. A coarse mesh was sent while a
neighbour at its level that holds ground or water was not, and it was seen
through the gap: a lake or sea bed, lit dark by the water over it (15 less 2,
then 3 a Voxel), and cave floors, lit as covered, drew black. A ray probe at
the black arc in `far` met level-1 `24,0,-15`, which holds the sea surface,
undelivered, then the delivered bed of `24,0,-16`. The orange dots are ore in
the stone beside the bed.

Fix, as the user chose (Q61): from level 2 up a level also meshes its hole,
the region the next finer level draws, and keeps each of those Chunks until
the finer levels have sent all of its region, then unloads it; the Client
draws the finer Chunks over it as they land (ADR-0019). Level 1's hole is
level 0's. A Chunk is sent once each neighbour it borders is sent, air, or
drawn by the next coarser level: a level further out stood so far off the
ground that a finer Chunk's side showed dark across the step. Where none of
those holds (the top level, or the next level not yet arrived) the Chunk
waits, with any neighbour whose region nothing else draws. Stand-ins do not
count for the hole-first order, or the finer levels stop filling coarsest
first. Test: `every_sent_chunk_borders_ground_that_some_level_draws`.
Captures in `/tmp/colour-06/v2`.

Measured against `79ec13cf`, dev builds, interleaved, load 3 to 11. Mid-load,
three pairs per view: black share 1.9% before, 0.00% (`far`) and 0.08%
(`jungle`, tree shadow, as settled) after. Time to levels 2 to 4 shown: `far`
42, 33, 32 s before, 38, 32, 33 s after; `jungle` 86 s before, 72 s after.
`flight` GPU frame (`--instruments gpu`, two series of three each): median
4.02 to 4.20 ms before, 3.73 to 3.77 ms after; p99 5.13 to 5.40 ms before,
4.66 to 4.76 ms after.

2026-11-25: Q74. The hybrid stays and is in the spec's Decisions: beside the
coarser level drawing its hole (Q61), a coarse Chunk waits until each
neighbour is sent, air, or drawn by the next coarser level. Only "hold until
every neighbour is sent" was rejected: it left large voids. `Level::holds`
and `covered_hole` now share `hole_covered_at`.
