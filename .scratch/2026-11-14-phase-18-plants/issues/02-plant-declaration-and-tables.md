# 02: Plant declaration and tables

Status: done

Blocked by: None (can start immediately)

Spec: [Phase 18](../spec.md).

## What to build

A Game or Mod can declare Plants as data and the engine loads them. The 0.3
contract gains the `plants` list with the five kinds and their common fields.
(`item-def`'s optional `texture` lands with ticket 13, which builds the channel
that delivers it, and `surface-feature`'s `plant` with ticket 12.)
Data-only Mods write `[[plants]]`. Names resolve after every Mod has declared;
the resolved Plant table, with the Stage-to-Plant lookup, lives beside the Item
and voxel-use tables in `tesserae-core`. Nothing grows yet.

## Acceptance criteria

- [x] `plants` appears in the 0.3 WIT, the host bindings,
      the data-only TOML and the validator; older lines are unchanged and a
      0.1 or 0.2 component still loads.
- [x] A load error names the Plant for an unknown Material, Item, tag or fluid,
      a Material in two Plants or as two Stages, a `harvest-to` that is not an
      earlier Stage, a Stage chain of one, a `mean-seconds` under the Sample
      interval, a size or limit out of range.
- [x] A later Stage Material and a tree's grown Materials get no implicit Item,
      and a load error names a grown Material that an Item or another Plant
      names. The seed Item places Stage 0.
- [x] A tree's `fruit` loads with 2 to 4 grown Stage Materials, a density from 0 to
      0.3 and a `harvest-to` that is an earlier Stage; fruit Materials are grown
      Materials and share none with another Plant.
- [x] A `tree` row's `sapling-yield` has a minimum of at least 1, else a load
      error.
- [x] Lookup by Material returns the Plant and Stage, and the next Stage.
- [x] The Engine ships no Plant: a World with no Game has none.
- [x] Unit tests beside the code cover every limit and error above.
