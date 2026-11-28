# 05: Flow and Falls from the Generator

Status: done
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

- [x] Tests: Flow points downstream along every stream; is zero on the sea;
      drifts toward the outlet on a lake.
- [x] Tests: a Fall across every lip with a lower pool past it, none across a
      lip beside dry ground, none elsewhere.
- [x] Tests: Flow is equal in any Chunk order and with a cold cache, and the
      same in metres at 1 m and 0.5 m Voxels; Falls are equal in any Chunk order
      and match each Voxel size's own lips.
- [x] Every recorded digest is unchanged.

## Comments

- 2026-11-26, on 5a4b7e69 plus this ticket:
  - `Worldgen::flow_at` asks the drainage outside its per-column loop, which
    stays as it was; only a wet column's mesh will ask it. On a stream it runs
    along the meandered centre line of the segment whose water stands highest,
    at Manning's speed from the segment's level drop and its dig, 0.3-4 m/s.
    On a lake it blends each corner node's way down its flood tree at 0.02-0.2
    m/s, faster with the node's catchment, so faster near the spill.
  - A lake wider than a window drained by each window's nearest edge, so two
    regions drifted it apart and the drift stalled at their border. The solve
    now marks the lake nodes that leave by a spill their window sees; the rest
    take the coarse solve's drift. Lakes held at the coarse solve's level can
    still drift toward its outlet across a fine shore, so the outlet test
    covers lakes their window sees whole (most of the test land's), and every
    drift must leave its lake.
  - `Worldgen::fall_at` is on the Voxel lattice: a lip is a column the
    containment raised, and a Fall runs to the side opposite a pool when the
    column there has a lower Water top. Of several, the longest drop.
  - The Fall tests use three 32 m boxes over steep brooks on terrain_v3's land
    at 1 m and 0.5 m.
  - Digests unchanged. The `worldgen_v3` bench is to rerun with the suite at
    the end: the node's lake flag is now a bit field.
