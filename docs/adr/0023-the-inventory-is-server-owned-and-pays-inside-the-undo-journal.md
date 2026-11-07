# The Inventory is Server-owned and pays inside the undo journal

Status: accepted.

Step 12 left Pickup feeding Carried, a count per Item with no slots and no
limits, and every placement free. Step 15 makes what a Player holds finite
without breaking the undo journal, which restores up to 128 groups of Edits and
applies each group all or nothing.

Each Player has an Inventory: a 27-slot backpack and a 9-slot Hotbar of Stacks,
each Stack an Item by name and a count up to the Item's declared `max-stack` (64
when absent). The rules (receive, count, spend, move, merge, swap, split, sort
and quick transfer) are a pure module in `tesserae-core`, so they are tested
without Bevy and run the same for every caller. The Inventory lives on the
Server's Player beside its game mode
([ADR-0024](0024-game-mode-is-per-player-with-a-world-default.md)) and its
Vitals. The Client sends intents (select a slot, move, split, quick transfer,
sort, take from the Catalogue, eat) and draws only what the Server sends back:
the whole Inventory when it changes, a refusal reason otherwise
([ADR-0002](0002-client-server-from-day-one.md)). Thirty-six slots are small
enough to send whole.

Every placeable Material is an Item. A Material that no Item names gets an
implicit Item of its own name, so fluid sources are placed from the Hotbar like
solids; fire stays a hand state. Shape and Orientation stay the Player's choice,
so one Stack of rock places cubes, slabs and stairs at one Item a Voxel.

In Survival the Server resolves Place from the selected slot, checks that its
Item places the intent's Material and spends one (Creative does the same since
the [amendment below](#amendment-2026-11-07-creative-spends-items-too)). A
Layout is paid up front: if the Inventory cannot pay for every Voxel, it is
refused before any Voxel changes. Pickup merges into matching Stacks, then fills
empty slots, Hotbar before backpack; what does not fit stays on the ground as a
smaller Dropped item.

Each undo entry records what its Edit did to the Inventory: nothing, the Item a
placement spent, or the Item and Dropped item id a removal produced. Restoring a
group checks the Items it needs together with the Voxels it expects, then
applies all of it or none of it. Undoing a placement refunds its Item; redoing
it spends one again and is refused without it. Undoing a removal takes its
Dropped item back if it still lies there, otherwise spends the Item and is
refused without it; redoing it drops the Item again. The cost is fixed when the
Edit is made, so an entry pays and refunds by the mode it was made in: an entry
made free stays free after a switch to Survival, and a Survival entry still
refunds in Creative. Reading the mode at undo time would let a switch mint or
destroy Items.

Anything that does not fit drops at the Player as Dropped items spread around
the body: an undo refund with no room, and the overflow of a save's Carried
totals when they are packed into slots at its first open. At death the whole
Inventory drops the same way, unless the Game declares `keep-inventory`, and the
undo history is cleared.

The Inventory is the Player extension `inventory_v1`, written in the same commit
as the World Edits it paid for, so a crash cannot keep one without the other.
Carried's `carried_v1` is read once and never written again.

Rejected: disabling undo in Survival, which would take away the building tool
players lean on most to avoid one Item check per entry; refusing an undo whose
refund has no room, which makes undo depend on an unrelated full backpack while
a drop at the Player's feet keeps the Item for the usual 5 minutes; and a Client
inventory synchronised to the Server, which ADR-0002 forbids and which
single-player would hide.

## Consequences

An implicit Item is an Item like any declared one: it drops as a quarter-Voxel
cube drawn as its Material, and Pickup takes it back, so death, an undo refund,
a redo and a save's Carried overflow lose nothing. Removing a fluid source drops
its Item as removing a solid does, so a source placed from the Hotbar can be
taken back, as Minecraft's and Luanti's buckets allow; flowing fluid drops
nothing. Only an Item that no content names has nothing to become; it is lost,
and a death does not report it. Throwing an Item away is the
[amendment below](#amendment-2026-11-07-dropping-an-item); containers and
crafting are later work.

## Amendment 2026-11-07: Creative spends Items too

The maintainer reversed the rule that placing is free in Creative, which this
ADR and [ADR-0024](0024-game-mode-is-per-player-with-a-world-default.md)
accepted. In every mode the Server now resolves Place and Layout from the
selected slot and spends one Item per Voxel; a Layout is paid up front, and a
Place from an empty slot is refused as NothingHeld ("Nothing to place in hotbar
slot N."). Undo and redo pay and refund Creative entries as they do Survival
ones.

Entries recorded before this keep the cost they were recorded with, as the
paragraph above already said, so a save made in Creative needs no migration. The
Catalogue still gives Materials only in Creative, which refills a spent slot.

A Place that names its own Voxel (`EditAction::Place`, `PlaceSource`) stays free
in Creative. The Client never sends it and nothing is gained by it: a Creative
Player can take any Material from the Catalogue at will, so it is not an economy
to guard, and refusing it would only break the Server's own tests and Mod
scenarios. Survival still refuses it.

The maintainer then made Creative removal drop too: breaking a solid or a fluid
source in Creative drops its Item as in Survival, with the same journal
bookkeeping (the entry records the Item and the Dropped item's id; undo takes
the Dropped item back if it still lies there; redo drops it again). A player who
broke a Terra melon in Creative and got nothing to pick up prompted it. Pickup
needs nothing different in Creative.

The old rules, "Creative placing is free" and, from
[ADR-0024](0024-game-mode-is-per-player-with-a-world-default.md), "in Creative
removing a Voxel drops nothing", are superseded.

## Amendment 2026-11-07: dropping an Item

A Player can throw a carried Item away from the Inventory window, which the
paragraph above listed as later work. The Client sends
`InventoryIntent::Drop { slot, count }`; the Server takes `count` out of that
Stack, refusing an empty slot, none, or more than the Stack holds, and tosses a
Dropped item a few metres ahead of the Player, past the pickup radius, in either
mode. An Item the World's content names no drop for is refused rather than lost.
The Dropped item cannot be picked up for four times a broken Voxel's delay, so
it is not collected while it is in the air; it lands out of reach, so standing
still never collects it, and walking to it after the delay does. A drop is not
an Edit and never enters the undo journal: nothing can be undone into the
Inventory by it, which is the rule that an entry pays and refunds only what its
own Edit did.
