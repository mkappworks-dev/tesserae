# Materials are table data, and light dimming belongs to them

Amended by [ADR-0016](0016-only-air-is-built-in.md): air is the only built-in
Material, ids 1 to 7 are retired, and the Game names its ground.

A Voxel is `(Material, Shape, Orientation)`.
[ADR-0006](0006-shapes-are-table-data.md) made the second a row in a table and
left a gap: a Shape answers geometry only, so a full cube always hid its
neighbours' faces and always stopped light, and glass, leaves and water had
nowhere to say otherwise.

**A Material is a row in a table, indexed by `MaterialId`.** A `MaterialTable`
holds what the engine asks of what a Voxel is made of: the block light it emits,
how many levels it dims light entering it (0 for glass, 15 for stone), whether
it hides the faces around it (`Opaque`, `SameMaterial` or `Never`), and which
render pass draws it. `emission` was a `match` on a placeholder id; it and the
new properties now share the one table.

**The `MaterialId` to definition mapping is therefore save data.** Ids 0 to 7
(air, stone, dirt, grass, lantern, glass, leaves, water) are permanent and never
renumbered (amended by [ADR-0015](0015-water-is-game-content.md): water is Game
content, the built-ins are ids 0 to 6 and id 7 is retired); a Mod's Materials
take ids from 8 in registration order, so a World needs the same name-to-id
registry in its save that Shapes need, and it is one registry for both. An id
the table does not know is drawn as an opaque stand-in, the opposite of an
unknown Shape, because a missing Material has a safe worst case and a missing
Shape does not.

**Dimming is the entered Voxel's, applied after the usual step, and geometry
only blocks light where dimming is 15.** An opaque cube is the case where the
arithmetic saturates, so it needs no special rule in the flood; a translucent
Voxel passes light through every face whatever its Shape and stores the light
that reaches it.

## Consequences

The Mod API hands over a list of `MaterialDef`s once at load, never a call per
Voxel, so [ADR-0001](0001-wasm-mod-boundary.md) holds by construction.
Appearance (colour, alpha, texture) is Look pack data and stays out of the
table; the render class is in it because the mesher, which knows nothing of
Bevy, decides which mesh a face goes into.

Until step 9, a Mod's Declaration supplies a flat appearance palette as a
stopgap, separate from the Material table (see
[ADR-0012](0012-mods-declare-once-and-command-in-batches.md)).

Terrain pays for none of this. Translucency is a per-Chunk gate read from the
palette, and the light flood and the mesher's first pass are specialised on it,
so a Chunk without a translucent Material runs the code that shipped before the
table existed.

A dimming is a count of attenuation steps and does not scale with Voxel size
([ADR-0003](0003-voxel-size-is-per-world.md)).
