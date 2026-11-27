# 25: Channels off the lattice

Status: ready-for-agent
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
