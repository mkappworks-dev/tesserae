# 12: Gates

Status: ready-for-agent

Blocked by: 09, 11, 13

Spec: [Phase 16](../spec.md), amendment 2026-11-14.

## What to build

A smaller Door for fences. Terra declares a Gate: a `door` row 1 m wide by 1 m
tall, with `gate` and `gate_open` Materials, a Gate Item, and a Workbench Recipe
(2 Wood). It places, toggles, breaks and undoes exactly as ticket 09's Doors do,
and uses the door Causes for sound. Its look matches Terra's fences: closed,
horizontal rails between two stiles; open, swung flush to one edge. The look
goes through the `/blocks` gallery for the user's approval before it is
committed, beside a fence run and the 1.8 m Player silhouette.

## Acceptance criteria

- [ ] A Survival player crafts a Gate at a Workbench, places it in a fence line,
      opens it, walks through and closes it.
- [ ] At 1 m and 0.25 m Voxels a Gate is 1 m by 1 m.
- [ ] The gallery shows the Gate closed and open beside a fence; the user
      approves the look before the Terra commit.
- [ ] Terra's Recipe count, Material count, texture fingerprint and appearance
      pins are updated.
