# 08: Spreaders

Status: done

Blocked by: 04

Spec: [Phase 18](../spec.md).

## What to build

The `spreader` kind: grass, moss, mushrooms and flowers copy themselves. A
Sample that passes picks a target within `reach-m` by hash and spreads there if
the target meets the Plant's soil and light rules, is Air above that soil, and
the `density` cap holds within `reach-m`. A mushroom's `light` is a ceiling as
well as a floor where the row says so.

## Acceptance criteria

- [x] A tuft spreads onto bare soil within its reach in metres and never onto a
      non-soil Voxel, a Voxel under a fluid or an occupied one.
- [x] The density cap holds: a field does not fill solid.
- [x] Mushrooms spread in the dark and not in light; grass the reverse.
- [x] Reach and rate are the same in metres and seconds at every Voxel size.
- [x] A bare Chunk beside a meadow grows only from the meadow's Samples, across
      the Chunk boundary into a held Chunk and never into an unheld one.

## Comments

- The target is one hash-picked Voxel among those whose centre lies inside the
  `reach-m` circle (`reach::spread_target`), plus a hash-picked step of -1, 0 or
  1 in height, so the chance per square metre of ground is the same at every
  Voxel size. The density cap counts the Plant's Material within `reach-m` of the
  target, a sphere, and an unknown cell counts as none, so beside an unheld
  Chunk a field may overshoot the cap by what it cannot see.
- The cap bounds the neighbourhood of each new Voxel when it lands, not of every
  older one, so a ring of Voxels can each sit under a cap whose centre is denser.
- A target must be Air with no fluid, no fluid above it, and pass `needs_met`
  (soil below, light band, Climate, water). An unheld or unlit Chunk fails it.
