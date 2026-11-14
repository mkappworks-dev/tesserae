# 08: Spreaders

Status: ready-for-agent

Blocked by: 04

Spec: [Phase 18](../spec.md).

## What to build

The `spreader` kind: grass, moss, mushrooms and flowers copy themselves. A
Sample that passes picks a target within `reach-m` by hash and spreads there if
the target meets the Plant's soil and light rules, is Air above that soil, and
the `density` cap holds within `reach-m`. A mushroom's `light` is a ceiling as
well as a floor where the row says so.

## Acceptance criteria

- [ ] A tuft spreads onto bare soil within its reach in metres and never onto a
      non-soil Voxel, a Voxel under a fluid or an occupied one.
- [ ] The density cap holds: a field does not fill solid.
- [ ] Mushrooms spread in the dark and not in light; grass the reverse.
- [ ] Reach and rate are the same in metres and seconds at every Voxel size.
- [ ] A bare Chunk beside a meadow grows only from the meadow's Samples, across
      the Chunk boundary into a held Chunk and never into an unheld one.
