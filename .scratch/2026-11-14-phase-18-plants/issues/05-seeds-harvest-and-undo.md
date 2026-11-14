# 05: Seeds, harvest and undo

Status: done

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

- [x] A Seed places Stage 0, is spent, and cannot place a later Stage; later
      Stages are not in the Catalogue.
- [x] Remove at a young, mid and ripe Stage drops each Stage's yield; Creative
      drops the same.
- [x] Use on a ripe `harvest-to` crop yields and resets, writes no journal
      entry, and refuses on an unripe one.
- [x] Use on a ripe fruit Voxel of a tree yields the Item (food) and resets the
      fruit without touching the tree; Remove of fruit drops its Stage's yield.
- [x] Undo of a Seed placement and of a removal still applies after the crop
      advanced a Stage; redo likewise, with Items paid and refunded.
- [x] A drop that does not fit the Inventory drops at the Player.

## Comments

Boxes ticked after review against `server/plants/tests.rs`: seed placement and spend in both modes, Remove at each Stage, Use harvest and fruit pick, Undo and Redo after growth, and a drop that does not fit.

- A removal's drops ride the entry as `Cost::Yielded(Vec<Drop>)` (item, count,
  Dropped item id). Undo takes back what still lies there and spends the rest
  from the Inventory, refusing whole without it; redo drops again. A plant
  Material that declares yields drops those and nothing else; one that declares
  none drops the Item that places it (a Seed at Stage 0), as before.
- `UseResult::Harvest` plays the existing `use-measured` Cause (the "Use
  answered" Sound); an unripe Voxel refuses as `UseRefusal::NotRipe`, which
  also stops Use falling through to eat.
- A picked or removed fruit leaves its tree's canopy; its undo entry restores
  the fruit Stage.
- A harvest's yield goes into the Inventory and what does not fit drops at the
  Player; a Remove's drops pop out of the Voxel as any removal's do.

