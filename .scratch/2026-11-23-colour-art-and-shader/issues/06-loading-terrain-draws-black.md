# 06: Loading terrain draws black

Status: ready-for-agent

Spec: [Colour, art and light](../spec.md).

## What to build

A bug on `main`, found in this spec's captures: while far LOD levels are still
loading, their terrain draws black with orange dots before it fills in. Seen
at `jungle-top-medium` and `far` with a 30 s settle under host load; level 1
held 0 to 54 Chunks where a settled view holds about 5340. The orange dots
suggest emissive Materials (lava) lit while the rest has no light yet, so a
Chunk may be drawn before its Sky light arrives. Diagnose first; that guess is
not confirmed.

Loading terrain either draws lit correctly or does not draw until it can.

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
draws the finer Chunks over it as they land (ADR-0019). A gap at a finer level
shows the coarser ground, not what lies under it. Level 1's hole is level 0's.
The top level, with nothing coarser, still sends its ground in groups that no
unsent ground borders, as does any level where the coarser one has not arrived.
Test: `every_sent_chunk_borders_ground_that_some_level_draws`. Captures in
`/tmp/colour-06`.
