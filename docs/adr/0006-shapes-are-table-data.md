# Shapes are table data, and an Orientation is one of 24 rotations

A Voxel is `(Material, Shape, Orientation)`, packed into a `u32` with 8 bits
each for the last two. Two decisions about those bytes are cheap now and
expensive once a World has been saved with them.

**A Shape is a row in a table, not a variant of an enum.** A Mod will want to
declare stairs, fences and nodeboxes of its own, and a Rust enum cannot grow at
load time. A `ShapeTable` holds each Shape as a union of boxes in sixteenths of
a Voxel, and everything the engine asks of one - what it hides, what light it
stops, what the mesher emits, what a body collides with, how much of its cell it
fills - is derived once at registration from one rotated 16×16×16 grid, so a
rotation bug is one bug and not three tables drifting apart.

**The `ShapeId` to definition mapping is therefore save data.** Ids 0 to 3
(cube, slab, stair, post) are permanent and never renumbered, and shape 0 can
never be redefined, because pass 1 of the mesher and the Chunk fast paths test
for it without a table. A Mod's Shapes are handed ids from 4 in registration
order, so a World needs a name-to-id registry in its save. A Voxel whose id the
table does not know is invisible and intangible, which is what a World looks
like when the Mod that defined it is missing.

**An Orientation is `up * 4 + turn`**, 0 to 23, with the top three bits reserved
and zero. `up` is which way the Shape's own +Y ends up, in the order +Y, -Y, +X,
-X, +Z, -Z so that byte 0 is the identity, and `turn` is quarter-turns about it.
One encoding serves every Shape: a slab's up and down, a stair's four facings
and an upside-down stair are all the same two fields. A byte of 24 or more reads
as the identity and is rejected on an Edit.

## Consequences

The Mod API hands over a list of `ShapeDef`s once at load, never a call per
Voxel, so [ADR-0001](0001-wasm-mod-boundary.md) holds by construction. Shapes
whose geometry depends on their neighbours (a fence growing arms) and collision
taller than the cell were not expressible here;
[ADR-0010](0010-connections-are-derived-not-stored.md) expresses both without
changing the Voxel's layout.

Anything physical derived from a Shape is a fraction of a Voxel and becomes a
distance only by multiplying by the World's Voxel size
([ADR-0003](0003-voxel-size-is-per-world.md)).

## Amendment 2026-11-14: flat forms for plants

A `ShapeDef` gains an optional `form`: `box`, the default and every Shape above,
`cross` or `plane`. A flat form is how a plant is drawn: `cross` is two diagonal
quads through the Voxel's centre, `plane` one quad along the first box's thin
axis, against a face, which a vine's Orientation turns to the wall that holds
it. The quads take the Material's own texture in the cutout pass, and the mesher
merges nothing across them and hides no neighbour's face behind them.

A flat Shape still declares boxes, derived once like any other, so the same grid
answers every other question:

- Its boxes decide selection, so a Player can aim at and break a plant they walk
  through, and where fluid can stand, so kelp is waterlogged where it grows
  ([ADR-0013](0013-fluid-and-fire-live-beside-the-voxel.md)).
- It is walked through unless it declares `collision`, which a cactus does.
- It seals no face: a box that covers one is refused at registration, so light
  and neighbours see past it, and a plant's Material is translucent to carry the
  dimming.
- It cannot connect: a `connect` on a flat form is refused
  ([ADR-0010](0010-connections-are-derived-not-stored.md)).
- It costs nothing against the Chunk's shaped-Voxel cap.

The ids and the registry are unchanged, and a Shape with no `form` is a `box`,
so a World saved before the forms has the Shapes it had. Plants are the reason
([ADR-0027](0027-plants-are-declared-kinds-the-server-grows.md)).
