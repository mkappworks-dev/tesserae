# 04: Face-direction split, prototype

Status: ready-for-agent
Blocked by: 01, 02, 07

Spec: [Packed quads](../spec.md).

## What to build

Item 2, as a measured prototype first: each Chunk layer's records sorted by
face, drawn as six entities, and the ones facing away from the Eye hidden.

Known costs to measure before any of it lands:

- Up to six times the entities and draws; on Metal a multi-draw is a loop of
  single draws.
- The sun's shadow pass draws from the sun, so faces the camera cannot see
  may still be needed there.
- Water is double-sided; its directions cannot be skipped.

## Acceptance criteria

- [ ] Ticket 01's numbers say draws are cheap enough to try.
- [ ] A prototype's A/B against 02 or 03, kept or dropped with the numbers
      here.

## Comments

2026-11-19: the user moved this to `ready-for-agent` after ticket 01. Gate the
prototype against the branch tip with ticket 07's fixes, by ticket 06's method.
