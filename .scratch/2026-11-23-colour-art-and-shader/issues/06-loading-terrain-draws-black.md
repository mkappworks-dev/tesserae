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

Fix: the Server withholds coarse meshes and sends a group once no Chunk in it
borders a non-air neighbour in its mesh shell that is not yet sent, unless a
coarser level draws that neighbour. Holding back for unmeshed neighbours only
left the same black one Chunk further on. Test:
`no_chunk_is_sent_beside_ground_its_level_has_not_sent`. Cost: a level's
ground arrives in larger groups, so mid-load views show more of the clear
colour where black was. Captures in `/tmp/colour-06`.
