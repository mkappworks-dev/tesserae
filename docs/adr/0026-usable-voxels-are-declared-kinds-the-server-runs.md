# Usable Voxels are declared kinds the Server runs

Status: accepted.

Step 16 lets Use act on a Voxel: a Door opens, a Container holds Stacks, a
Station offers Recipes. A Game or Mod declares which Materials are usable as
data, one `voxel-uses` row each with a closed kind (`door` with an open
Material, `container` with a slot count, `station` with a tag), and the Server
runs each kind in native code. No Mod is called when a Voxel is used, as for
Tools ([ADR-0001](0001-wasm-mod-boundary.md)); a new kind needs new engine
mechanism and arrives with the contract line that adds it.

Recipes are data too. Each names where it is crafted, `"hand"` or a Station tag;
the field is required and `hand` is a reserved tag, so a forgotten field is a
load error and not an accidental hand Recipe. Recipes name a tag, not a
Material, so a Mod's Material that serves an existing tag opens existing
Recipes. Crafting is instant and shapeless, with one output.

The Engine draws every pane: the Crafting tab, a Station's Recipes and a
Container's slots, from declared data and what the Server sends. A Mod supplies
no UI, so nothing crosses the contract per frame or per click.

A Door's state is its Material: Use swaps the closed and open Materials of the
whole Door, a size in metres (see the amendment below), as an Edit that never
enters the undo journal. Saving, meshing and light need nothing new. Collision
needs one rule: `Tables::blocks` lets bodies pass through a Door's open
Material, while rays still stop on it, so it can be aimed at, used and broken.
The open Material gets no implicit Item and cannot be placed; removing it drops
the closed one's Item.

A Container's Stacks live outside the Chunk, keyed by Voxel position, and are
saved as an additive list in the World commit. A Player's Remove of a Container
that holds Stacks is refused, and so is an undo or redo that would remove one.
Anything else that removes it spills the Stacks as Dropped items: the Server
checks every held Container each tick, and one whose Voxel is no longer a
Container spills there, as does what no longer fits when a smaller Container
replaces it.

Station and Container sessions close when the Voxel changes or the Player leaves
Edit reach of it, measured from the Eye to the Voxel's nearest point, so a
session opened at the edge of reach does not close at once.

Rejected:

- A Mod callback on Use, which is the per-use host call ADR-0001 forbids.
- Mod-drawn UI, which would need a UI contract and run guest code every frame.
- Door state as per-Voxel data beside the Material, which every Chunk, save and
  mesh path would have to carry for one bit a second Material already expresses.
- Spilling a Container on the Player's Remove, as Minecraft does: undo would
  then have to restore Stacks into a re-placed Container, a new kind of journal
  payload
  ([ADR-0023](0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md)).
- Recipes that leave out `station` for hand crafting: an omitted field cannot be
  told from a forgotten one.

## Consequences

Container sessions are the first per-Voxel Server state outside the Chunk. The
Player's own Remove and history check for a Container and refuse. Spilling is
polled rather than hooked into each path that removes or replaces a Voxel (edit,
Mod Commands, burning), so a new path is covered without change, at the cost of
a lookup per held Container per tick.

## Amendment 2026-11-14: a Door is a size in metres

A Door row declares a width and height in metres, 1 m by 2 m when absent, from
0.5 to 4 m wide and 1 to 4 m tall, so a Door is taller and wider than the Player
at every Voxel size ([ADR-0003](0003-voxel-size-is-per-world.md)); the run of at
most 4 Voxels stood 1 m tall at 0.25 m. One Item places the whole Door, ⌈size ÷
Voxel⌉ cells one Voxel thick across the Player's facing, as one undo group that
is refused whole if any cell is blocked. Use toggles every cell, and removing
any cell removes the whole Door as one undo group that drops one Item.

Which cells are one Door is derived, not stored, as Connections are
([ADR-0010](0010-connections-are-derived-not-stored.md)): the cells of that
Door's Materials that touch in its plane are tiled Door-wide from their lowest
corner, then each column of tiles Door-tall from its own lowest cell. Its plane
is the horizontal axis the Material runs further along through the aimed cell.
So Doors placed side by side, even on a step, are two Doors; Doors stacked out
of line across are not told apart, and a wall of one Door's Material built
another way is tiled the same way.

Rejected:

- A Door identity stored per Voxel or beside the Chunk, which every save, undo
  and Mod Command path would have to keep in step for what the cells already
  show.
