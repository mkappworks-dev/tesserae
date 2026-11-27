# 22: Stepped river profile and real waterfalls

Status: done
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

- 2026-11-27: a segment dropping 4 m or more at 10% or steeper holds its
  upper level and drops down one face 3.5 in 1, at a hashed point between 0.3
  and 0.7 along it; the heightfield stays a slope. A Fall's pool may lie up
  to three columns past its lip, and its landing foam is the first wet one.
  Each lip carries a drawn film a sixteenth over its top, running toward the
  Fall. Falls of 13 m off the sea cliffs near -5658, -2035:
  `/tmp/tesserae-water-look-captures/24-after/fall.png`. The coarse digests moved.
- 2026-11-27, the look. A Fall's wall read as dirty glass: short grey dashes
  with the rock behind. Falling water is full of air, so at least 0.6 of the
  wall is white, in long fine strands 0.8 m across and 12 m down, falling at
  6 m/s. `/tmp/tesserae-water-look-captures/22-falls2/fall.png`.
