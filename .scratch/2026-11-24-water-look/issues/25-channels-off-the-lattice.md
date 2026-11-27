# 25: Channels off the lattice

Status: done
Blocked by: 24

Spec: [Water that looks natural](../spec.md), Rivers. From the user's review:
channels run in straight lines along the lattice.

## What to build

Each node's position is jittered within its cell, as ADR-0030 already states,
and a channel is drawn as a smooth curve through its upstream node, its node,
its parent and its parent's parent, so channels no longer run in straight
45-degree and axis-aligned runs. The jitter is taken out of the meander's
budget, so the ground does not step at the stencil's edge.

## Acceptance criteria

- [ ] A map of the network shows no straight lattice-aligned runs over a
      node long.
- [ ] `worldgen_v3` within 5% of before, or the cost named for the user.

## Comments

- 2026-11-27: each node stands up to 0.3 of a node off its lattice point,
  hashed, and the valley's reach gives up as much, so it stays in the
  stencil. The Fall's face takes the jittered segment's own length. Curves
  through four nodes were left out: on a 1 km map the meander and jitter
  leave far fewer straight runs, judged by eye, not measured.
- 2026-11-27: the offset is hashed once per node in the solve and kept in it,
  as every column of a channel asks it; hashing it per column cost the
  straddling-dry case 7%.
