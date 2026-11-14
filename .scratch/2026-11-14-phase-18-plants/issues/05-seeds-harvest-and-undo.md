# 05: Seeds, harvest and undo

Status: ready-for-agent

Blocked by: 04

Spec: [Phase 18](../spec.md).

## What to build

A player plants a Seed from the Hotbar, breaks a plant for its Stage's yield,
and picks a ripe regrowing crop with Use. Remove drops the Stage's declared
yield, each count a hash of seed, position and Stage within its range, in both
modes, and rides in the undo entry as any removal does. Use on a ripe crop with
`harvest-to` yields and resets the Voxel, as an Edit the journal never records;
`use_aim` reports Harvest and the existing "Use answered" Sound plays. Undo and
redo treat all Stages of one crop as the same Voxel.

## Acceptance criteria

- [ ] A Seed places Stage 0, is spent, and cannot place a later Stage; later
      Stages are not in the Catalogue.
- [ ] Remove at a young, mid and ripe Stage drops each Stage's yield; Creative
      drops the same.
- [ ] Use on a ripe `harvest-to` crop yields and resets, writes no journal
      entry, and refuses on an unripe one.
- [ ] Use on a ripe fruit Voxel of a tree yields the Item (food) and resets the
      fruit without touching the tree; Remove of fruit drops its Stage's yield.
- [ ] Undo of a Seed placement and of a removal still applies after the crop
      advanced a Stage; redo likewise, with Items paid and refunded.
- [ ] A drop that does not fit the Inventory drops at the Player.
