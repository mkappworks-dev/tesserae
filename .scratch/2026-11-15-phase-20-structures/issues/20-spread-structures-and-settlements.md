# 20: Spread Structures and Settlements evenly but irregularly

Status: ready-for-agent

Blocked by: 03, 14, 15

Spec: [Phase 20](../spec.md).

## What to build

Structures and Settlements are spread across a World, and buildings across a
Settlement, by a blue-noise scheme drawn from the World seed: never in rows or
on a visible grid, never clumped, and different in every World.

Today a Settlement's centre is jittered only by the slack between its lattice
spacing and twice its radius plus margin. At the smallest allowed spacing that
slack is zero, so centres stand on an exact grid, and every listed cell holds
one. Inside a Settlement, Plots are packed every 2 m along each road, nearest
the centre first, so every village is a tight cluster.

- **Across the World**, for single Structures and Settlement centres alike:
  - Each lattice cell draws a candidate anywhere in the cell from the seed.
  - A candidate stands only if no candidate in a neighbouring cell with a
    higher draw lies within the minimum distance. The minimum distance is the
    declared spacing for Structures, and the reach for Settlements.
  - This is a pure-function Poisson-disc scheme: each cell asks only its
    neighbours' candidates, never the ground rules, so it stays cheap and
    independent of Chunk order.
  - A Biome's Settlement row gains a `chance`, as a Structure row already has,
    so not every cell holds one.
- **Inside a Settlement**, Plots are spread over the whole disc rather than
  packed from the centre out:
  - Slots along the roads are tried in a seeded order with seeded gaps, so
    there is open ground between houses.
  - Each Plot sits a seeded distance back from the road, within a range.
  - The density falls off towards the edge, so outer buildings stand further
    apart.
  - Roads still branch from the centre, and doors still face their road.
- Everything is computed in metres from the seed and the cell, and is
  identical at every Voxel size.

## Acceptance criteria

- [ ] No two Settlements' discs plus margins overlap, and no two Structures
      of one row stand closer than their spacing, across a large scan in any
      Chunk order.
- [ ] Settlement centres in a large scan show no grid: the spread of their
      positions within their lattice cells is close to uniform, tested.
- [ ] Two seeds give different layouts; one seed gives the same layout in
      any Chunk order and at 1.0, 0.5 and 0.25 m.
- [ ] Within a Settlement, the gaps between neighbouring Plots vary, and
      Plots reach the outer half of the disc.
- [ ] WIT, TOML, validator and snapshot carry the Settlement row's `chance`,
      and the WIT copies are synced.
- [ ] The benchmark (ticket 18) still meets its margin.
