# 01: Declare items

Status: done

Blocked by: None (can start immediately)

## What to build

A Game or data Mod declares items and the Server holds them. `items` is
appended to the unreleased 0.3 `declaration`, with `[[items]]` in a data
Declaration. An item has a local name, `size-m`, a display `color` and an
optional solid `material`. A bad item fails the load naming the Mod and the
field. Lines 0.1 and 0.2 adapt to none. The Server exposes the resolved items
(id by registration order, name `<mod id>:<name>`) as a resource.

## Acceptance criteria

- [x] A data Mod with `[[items]]` loads and its items are held by the Server in
      registration order with full names.
- [x] Refused, each naming Mod and field: duplicate name, unknown or non-solid
      `material`, two Items on one Material, `size-m` outside 0.05–0.5, over 64
      per Mod or 256 per World.
- [x] A 0.1 and a 0.2 component still load, with no items.
- [x] The starter `wit/` copy, authoring guide schema, data-mods doc and
      `docs/contract-compatibility.md` are updated by appending.
- [x] GLOSSARY gains Item, Entity, Dropped item and Carried, and the Pickup
      entry is redefined.
- [x] Tests at the Declaration-loading seam.
