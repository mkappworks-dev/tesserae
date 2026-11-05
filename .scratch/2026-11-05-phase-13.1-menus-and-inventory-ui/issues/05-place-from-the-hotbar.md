# 05: Place from the hotbar's Items

Status: done

Blocked by: 04

Spec: [Phase 13.1](../spec.md).

## What to build

The hotbar in play is the Inventory's hotbar. Digits 1-9 and the scroll wheel select a slot through the Server. Place resolves the selected slot's Item on the Server and places the Material that Item places, with the Client's chosen Shape and Orientation (E still flips). A Material with no declared Item gets an implicit Item of the same name, so fluid sources are Items too; Ignite stays a Tool. The Client hotbar and its per-Game shortcuts are retired in favour of the Server hotbar. The hotbar takes the prototype look: recessed bevel slots, a bright rim on hover, a lit frame on the selected slot, and the held Item's name above it. Placing is still free (ticket 06 adds Survival).

## Acceptance criteria

- [x] Placing uses the selected slot's Item and the chosen Shape and Orientation.
- [x] A fluid source can be placed from its implicit Item.
- [x] The hotbar and the held Item's name follow Server messages only.
- [x] Existing placement, Layout and Sample journeys still work.
