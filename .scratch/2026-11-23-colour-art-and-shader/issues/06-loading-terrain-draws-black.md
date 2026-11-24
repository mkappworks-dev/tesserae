# 06: Loading terrain draws black

Status: ready-for-agent

Spec: [Colour, art and light](../spec.md).

## What to build

A bug on `main`, found in this spec's captures: while far LOD levels are still
loading, their terrain draws black with orange dots before it fills in. Seen
at `jungle-top-medium` and `far` with a 30 s settle under host load; level 1
held 0 to 54 Chunks where a settled view holds about 5340.

Cause (diagnosed): a coarse Chunk was sent before a ground or water neighbour
at its level, and the dark beds and cave floors beneath showed through the gap.
Fix (user, Q61): a coarser level keeps drawing a region until every finer Chunk
covering it is sent. Holding Chunks back until their neighbours are sent was
rejected: it left larger empty ground mid-load.

## Acceptance

- [ ] The cause named, with a reproduction (a test where the cause allows one).
- [ ] A capture taken mid-load at `jungle-top-medium` and `far` shows no black
      terrain.
- [ ] No regression in time to a settled view; `flight` GPU frame no worse than
      `main` beyond noise.
