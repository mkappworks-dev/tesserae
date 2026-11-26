# 11: Running rivers

Status: ready-for-agent
Blocked by: 10

Spec: [Water that looks natural](../spec.md), The water path.

## What to build

A river's surface runs downstream. The normal map is distorted along each
column's Flow in two layers half a period apart, cross-faded, with a noise
offset per column to hide the pulse; normal strength and foam grow with speed,
so steep reaches show white water. Lakes drift slowly toward their outlets.
Poured and simulated water runs by the same rule. Flow-phase periods are whole
multiples of the clock's, as in 08.

## Acceptance criteria

- [ ] Captures over several seconds at the river fixture show the surface moving
      downstream with no visible pulse or reset, and white water on steep
      reaches; shown to the user.
- [ ] Water poured on a slope runs down it in a capture.
- [ ] Flow fades to calm water by the edge of level 0.
- [ ] The frequency unit test from 08 covers the flow phase.
