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
