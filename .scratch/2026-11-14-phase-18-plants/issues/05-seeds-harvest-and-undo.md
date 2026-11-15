# 05: Seeds, harvest and undo

Status: done

Blocked by: 04

Spec: [Phase 18](../spec.md).

## What to build

A player plants a Seed from the Hotbar and breaks a plant for its Stage's yield;
breaking a ripe regrowing crop resets it. Remove drops the Stage's declared
yield, each count a hash of seed, position and Stage within its range, in both
modes, and rides in the undo entry as any removal does. Remove of a ripe crop with
`harvest-to` yields and resets the Voxel, as an Edit the journal never records. Undo and
redo treat all Stages of one crop as the same Voxel.

## Acceptance criteria

- [x] A Seed places Stage 0, is spent, and cannot place a later Stage; later
      Stages are not in the Catalogue.
- [x] Remove at a young, mid and ripe Stage drops each Stage's yield; Creative
      drops the same.
- [x] Remove of a ripe `harvest-to` crop yields and resets, writes no journal
      entry; of an unripe Stage it breaks. Use harvests nothing.
- [x] Remove of ripe fruit drops its yield (food) and leaves the tree's canopy;
      of unripe fruit drops nothing; undo and redo restore it and the drops.
- [x] Undo of a Seed placement and of a removal still applies after the crop
      advanced a Stage; redo likewise, with Items paid and refunded.
- [x] A drop that does not fit the Inventory drops at the Player.

## Comments

Boxes ticked after review against `server/plants/tests.rs`: seed placement and spend in both modes, Remove at each Stage, a crop harvest and a fruit break, Undo and Redo after growth, and a drop that does not fit.

- A removal's drops ride the entry as `Cost::Yielded(Vec<Drop>)` (item, count,
  Dropped item id). Undo takes back what still lies there and spends the rest
  from the Inventory, refusing whole without it; redo drops again. A plant
  Material that declares yields drops those and nothing else; one that declares
  none drops the Item that places it (a Seed at Stage 0), as before.
- `UseResult::Harvest` plays the existing `use-measured` Cause (the "Use
  answered" Sound); an unripe Voxel refuses as `UseRefusal::NotRipe`, which
  also stops Use falling through to eat.
- Superseded: Remove of a ripe fruit or a ripe `harvest-to` crop is a harvest,
  the code path of Use (`plants::harvest`, reached from `apply_pending`): yield
  into the Inventory, Voxel reset to `harvest-to`, no journal entry. Remove of
  an unripe fruit still leaves canopy (its undo entry restores the fruit Stage);
  an unripe crop Stage breaks as before, which is how a berry bush is removed.
- A harvest's yield goes into the Inventory and what does not fit drops at the
  Player; a Remove's drops pop out of the Voxel as any removal's do.
- Reversed by the maintainer ("pressing G to harvest is confusing; the only way
  to harvest is by breaking it"): `Usable::Harvest`, `UseResult::Harvest`,
  `UseRefusal::NotRipe`, the `G Harvest` prompt and Use's Harvest step are gone;
  Use is a Tool, a Usable Voxel, then eating. Only a ripe crop with `harvest-to`
  still resets on Remove (`plants::harvest`, no journal entry).
- A fruit break is a normal Remove: the yield is Dropped items at the Voxel
  (`Cost::Yielded`, so undo takes them back), not the Inventory-first path of
  the crop harvest, and `fruit-def` has no `harvest-to`; new fruit comes from
  re-bearing (ticket 10).
