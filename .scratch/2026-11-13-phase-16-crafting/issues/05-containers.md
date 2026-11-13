# 05: Containers

Status: ready-for-agent

Blocked by: 01, 03

Spec: [Phase 16](../spec.md).

## What to build

A player places a Chest, opens it with G, moves Stacks in and out, leaves, and
finds them there after reopening the World. `voxel-uses` gains the `container`
kind with `slots` (1 to 54, 27 when absent). Each Container Voxel has its own
Stacks, saved as an additive `containers` list in the World commit (absent means
none). Use opens a session; the Server sends the Container's Stacks whole to
every viewer when they change. Inventory intents take a slot address naming an
Inventory or a Container slot, using ticket 01's shared rules; Take all and Sort
act on the Container; none of it enters the journal. Sessions close on leaving
reach or on the Voxel changing. A Player's Remove of a Container that holds
Stacks is refused ("Empty the Chest first."), and so is an undo or redo that
would remove one; any other removal or replacement (a Mod Command, burning)
spills the Stacks as Dropped items. The Client opens the Container grid beside
the Inventory as the mockup shows. Terra's Chest becomes a Container.

## Acceptance criteria

- [ ] Move, split, merge, swap, quick transfer, Take all and Sort work between a
      Container and the Inventory.
- [ ] Two sessions on one Container see the same contents after either changes
      it.
- [ ] Remove, undo and redo of a non-empty Container are refused; of an empty
      one they behave as for any Voxel.
- [ ] A Mod Command or burning that replaces a full Container spills every
      Stack.
- [ ] Contents survive reopening; a commit without `containers` opens; an
      orphaned entry spills at open.
- [ ] Leaving reach or the Container changing closes the pane with a notice.
