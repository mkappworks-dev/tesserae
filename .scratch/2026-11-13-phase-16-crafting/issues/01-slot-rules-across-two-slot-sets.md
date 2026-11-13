# 01: Slot rules work across two slot sets

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Phase 16](../spec.md).

## What to build

A prefactor with no behaviour change. The pure Inventory rules in
`tesserae-core` (move, merge, swap, split, quick transfer, sort) work across any
two slot sets, so a Container's slots can later share them with the Inventory.
The Inventory keeps its backpack and hotbar and behaves exactly as today;
nothing outside `tesserae-core` changes behaviour.

## Acceptance criteria

- [ ] Move, merge, swap, split, quick transfer and sort are expressed once over
      a pair of slot sets.
- [ ] A unit test moves, merges and quick-transfers between an Inventory and a
      second slot set of another length.
- [ ] Every existing Inventory, Server and save test passes unchanged.
