# 07: The Quality curve on the Server

Status: ready-for-agent
Blocked by: 01

Spec: [LOD features](../spec.md).
[ADR-0036](../../../docs/adr/0036-the-quality-curve-is-a-player-preference-the-server-clamps.md).

## What to build

The Server streams the coarse rings from a Quality curve instead of a fixed
doubling: a unit `u` and a reach, so coarse level k ends at
`min(u * 2^k, reach)`. The Client requests a curve; the Server clamps the unit
to at least `view_distance_m` and the reach to the Game's `lod_distance_m`, and
streams from it. Level 0 stays the Game's `view_distance_m`. The level count,
holes and overlaps are derived for each pair of widths by the existing rules,
checked for all four parities. Until 08, a `dev` switch sets the curve.

Presets: Balanced (`u` 192 m) and Detailed (`u` 384 m); Custom is any unit from
192 to 384 m and reach from 2 to 8 km.

## Acceptance criteria

- [ ] Ring tests for both presets, a Custom curve and the clamp, at all four
      parities; Balanced equals today's rings exactly.
- [ ] Level 0's rings are unchanged by any curve.
- [ ] Changing the curve at run time streams the new rings with no gap
      (ADR-0019), in a Bevy app test.
- [ ] Captures with the debug tint at Balanced and Detailed, for 09's review
      batch.
