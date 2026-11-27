# 22: Stepped river profile and real waterfalls

Status: ready-for-agent
Blocked by: 21

Spec: [Water that looks natural](../spec.md), Rivers. From the user's review:
rock lips read as dams; they want tall vertical waterfalls.

## What to build

A river's surface steps by its slope class instead of a straight line between
nodes: long pools and riffles below 2%, rapids to 4%, step-pools to 10%, and
one tall Fall past that where the drop is at least 4 m. A Fall stands where
the ground is steepest, and where a tributary hangs over a deeper valley; it
has a vertical face, a sheet of water down it and a plunge pool under it. A
lip never stands above the water over it, reads as gravel or the stream bed,
and carries a spill sheet, so no step reads as a dam.

## Acceptance criteria

- [ ] No lip stands above the upper pool's surface.
- [ ] A Fall of 4 m or more on the slope fixture, with a plunge pool; captures
      shown to the user.
- [ ] Containment still leaves no generated water beside open air, except the
      Fall's own face.

## Comments
