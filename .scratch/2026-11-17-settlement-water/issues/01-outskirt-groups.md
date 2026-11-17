# 01: Outskirt groups

Status: ready-for-agent

Spec: [Water by Settlements](../spec.md).

## What to build

`Settlement.outskirts` becomes a list of groups, each with its own chance, count
and salt, laid in order. The contract, loader and save follow; a saved single
group restores as one.

## Acceptance criteria

- [ ] A second group lays its Templates and leaves the first group's Plots where
      they were.
- [ ] A save with the earlier single `outskirts` object restores as one group.
