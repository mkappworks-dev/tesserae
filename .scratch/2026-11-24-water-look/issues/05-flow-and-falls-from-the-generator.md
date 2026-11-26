# 05: Flow and Falls from the Generator

Status: ready-for-agent
Blocked by: 04

Spec: [Water that looks natural](../spec.md), Flow and Falls from the Generator.

## What to build

The Generator answers each column's Flow and where each Fall is, beside its
Water surface answer.

- **Flow**, pure in the seed and metres. On a river it follows its segment,
  turned by the meander so it follows the bends, at a speed from the segment's
  slope and catchment, clamped to a natural range. On a lake it is a slow drift
  toward the outlet, faster near the spill; the sea has none.
- **Falls**, pure in the seed, metres and Voxel size, since the lips are asked
  on each Voxel size's own lattice. A lip is a dry column whose ground was
  raised to hold a higher pool beside it; a Fall runs from that pool over the
  lip to the next column past it, when that column has a lower Water surface. A
  lip with dry ground past it has no Fall.

New code uses no platform maths (ADR-0032). No terrain changes.

## Acceptance criteria

- [ ] Tests: Flow points downstream along every stream; is zero on the sea;
      drifts toward the outlet on a lake.
- [ ] Tests: a Fall across every lip with a lower pool past it, none across a
      lip beside dry ground, none elsewhere.
- [ ] Tests: Flow is equal in any Chunk order and with a cold cache, and the
      same in metres at 1 m and 0.5 m Voxels; Falls are equal in any Chunk order
      and match each Voxel size's own lips.
- [ ] Every recorded digest is unchanged.
