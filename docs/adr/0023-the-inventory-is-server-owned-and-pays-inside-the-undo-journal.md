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
Item places the intent's Material and spends one. A Layout is paid up front: if
the Inventory cannot pay for every Voxel, it is refused before any Voxel
changes. Pickup merges into matching Stacks, then fills empty slots, Hotbar
before backpack; what does not fit stays on the ground as a smaller Dropped
item.

Each undo entry records what its Edit did to the Inventory: nothing, the Item a
placement spent, or the Item and Dropped item id a removal produced. Restoring a
group checks the Items it needs together with the Voxels it expects, then
applies all of it or none of it. Undoing a placement refunds its Item; redoing
it spends one again and is refused without it. Undoing a removal takes its
Dropped item back if it still lies there, otherwise spends the Item and is
refused without it; redoing it drops the Item again. The cost is fixed when the
Edit is made, so an entry pays and refunds by the mode it was made in: a
Creative entry stays free after a switch to Survival, and a Survival entry still
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
a redo and a save's Carried overflow lose nothing. Only an Item that no content
names has nothing to become; it is lost, and a death does not report it.
Throwing an Item away, containers and crafting are later work.
