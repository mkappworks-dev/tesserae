# Tesserae — glossary

Canonical vocabulary. Terms here are the only names used in code, docs and the
mod API. This file is a glossary: no decisions, no implementation.

See [DESIGN.md](DESIGN.md) for decisions and [docs/adr/](docs/adr/) for the ones
that are hard to reverse.

## Content

**Engine** — Tesserae itself. Ships no gameplay.

**Game** — a Mod whose manifest declares it a root. There is one loader and one
format; a game is not a separate kind of thing.

**Mod** — a unit of content loaded by the engine. Declares block behaviour,
entities, recipes, worldgen parameters, assets.

**Look pack** — a Mod that supplies rendering appearance only: materials and
post-process settings. The equivalent of a Minecraft shaderpack.

**Manifest** — a Mod's identity, version, contract line, dependencies and
requested limits; marks whether it is a Game.

**Contract line** — one incompatible version of the WIT contract a Mod is built
against, `0.1`, `0.2` or `0.3`. Patches within a line stay compatible; a closed
record or variant that grows needs a new line.

**Adapter** — the host's translation of a component built against an older
contract line to and from the next one, chained up to the current one, so it
keeps loading unchanged.

**Declaration** — the content a Mod supplies when loaded: Materials, Shapes,
appearance, fluids, reactions, combustion and, for a Game, World parameters.

**Registry** — the correspondence between names and the ids of registered
Materials and Shapes.

**Trust** — a user's authorization to run a particular build of a Mod.

## World

**World** — one save. Owns its Voxel size, its seed and its Chunks.

**World list** — the first screen of a launch: every World folder, before any
Game runs. Not a "menu" (the in-World catalogue and settings).

**Voxel** — the atomic unit of world volume. Not "block", not "node".

**Material** — what a Voxel is made of. Stone, water, oak. A row of game data:
the light it emits and the light it dims, which faces it hides, which mesh it is
drawn in, and whether it is solid. Not its appearance: colours and textures are
separate Declaration data, shared by World surfaces and item previews until a
Look pack supplies appearance. Bevy's `Material` and `MaterialExtension`, which
shade a surface, are a different thing.

**Shape** — a Voxel's geometry. A full cube, a stair, a slab, a fence. "A stone
voxel" and "a stone stair" share a Material and differ in Shape, so neither word
alone identifies a Voxel.

**Orientation** — a Shape's rotation: one of the 24 axis-aligned rotations.
Meaningless for a full cube, which looks the same under all of them.

**Nodebox** — an axis-aligned box inside one Voxel, in sixteenths of its edge. A
Shape is a union of them.

**Connection** — an arm a connected Shape grows towards a face-neighbour because
of what that neighbour is. Derived from the six Voxels around, never stored in a
Voxel.

**Connection group** — one of 32 bits a Shape can belong to and can connect to.
A property of a Shape, not of a Material.

**Overhang** — how far a Shape's collision extends beyond its own cell, in
Voxels, at most one.

**Sealed face** — a face of a Voxel that its Shape covers completely. What
occlusion and light both ask about; derived from geometry, never authored.

**Dimming** — the levels of light a Material removes from light entering a Voxel
of it, on top of the one level every step already costs. 0 for air and glass, 15
for stone. A count of attenuation steps like every light quantity, so it does
not scale with Voxel size.

**Opaque** — a Material that hides every face of its neighbours and dims light
by 15. Stone.

**Translucent** — any Material that is not Opaque: glass, leaves, water. What a
Translucent Material hides, how much it dims and which render pass draws it are
properties of the Material, never of its Shape.

**Voxel size** — the edge length of one Voxel in metres. A property of a World,
not of the engine. Default 1.0.

**Air** — the absence of Material. A Voxel, not a null. The only built-in
Material, id 0; ids 1 to 7 are retired.

**Ground** — the Game's optional rock, soil and surface Materials, named in
World parameters. Without ground the World is empty air.

**Cave** — a tunnel of Air that World generation carves through solid ground.
Sized in metres, so it is the same Cave at any Voxel size. Where one breaks the
surface it has a **mouth**, and sky light can enter there.

**Water level** — the height in metres up to which World generation fills the
open air above the ground with the Game's lake fluid, any fluid it declared. One
level for the whole World, so a valley below it is a lake; a Cave is never
filled. A Game that names no lake fluid, and a World with no Game, have none.

**Light level** — how much light reaches a Voxel, from 0 to 15. The one quantity
in the engine that is **dimensionless**: a count of attenuation steps, not a
distance, so it does not scale with Voxel size the way everything physical does.

**Sky light** — the light a Voxel receives from the sky. At full strength it
falls straight down undimmed, so open ground is lit the same however tall the
air column above it is.

**Block light** — the light a Voxel receives from Materials that emit. Dims by
one level per Voxel in every direction, downwards included.

## Space

**Chunk** — the fixed-size cubic array of Voxels that storage, meshing and
streaming all operate on. The same count of Voxels at every LOD level.

**Cell** — a Chunk's footprint in metres. Grows with LOD level, unlike a Chunk.
Use "Cell" whenever the unit is distance and "Chunk" whenever it is data.

**LOD level** — how many Voxels of the level below one Voxel of this level
aggregates. Level 0 is full detail.

**LOD ring** — what one LOD level draws: the Cells between an inner and an outer
distance from the Eye, in metres. The outer distance doubles with each level.

**Distance fog** — how much of a surface's colour is replaced by the fog colour,
as a function of its distance from the Eye in metres. Sized from the full-detail
radius and the reach, so it is the same fog at any Voxel size. The fog colour
and the colour of the sky behind the terrain are two values, not one.

**Downsample** — producing a coarse Chunk's Voxels from the eight finer Chunks
it covers: the dominant Material by volume, solid if half the volume is. At
level 1 fluid weighs the volume it fills.

**World generation** — producing a Chunk's contents from a seed and a position.
Never produces a mesh.

## Runtime

**Server** — the authority over World state. The only thing permitted to mutate
a Chunk.

**Client** — renders and sends intent. Never mutates World state.

**Integrated server** — a Server running in the Client's process, for single
player. Not a different Server.

**Player** — what the Server holds for each person in the World, including where
that person is. The Client sends movement intent; the Server advances the body
and publishes its Eye. Movement mode and camera view are independent.

**Eye** — the point in metres a Player sees and acts from. An Edit's ray starts
near it or at a third-person camera, but reach and sight are judged from the
Eye. Not the feet and not a body.

**View mode** — whether the Client draws from the Eye (first-person) or from
behind the Player's shoulder (third-person). A user's preference, not World
state, and never a source of movement or reach.

**Boom** — how far behind the Player's shoulder a third-person camera sits, in
metres. Drawn terrain shortens it; nothing lengthens an Edit's reach along it.

**Edit** — a change to one Voxel, its solid or its source, applied by the
Server. A Client sends an intent; it never makes an Edit.

**Use** — a Player pressing a held Tool's action. Answered by the Server, never
applied: it changes no Voxel and enters no history. Not an Edit, not Sample, and
not Pickup.

**Tool** — a thing a Player holds whose Use the Server runs, declared by the
Game by name and kind. A hand state taken with its own key, not from the
Catalogue: while one is held, Place and Remove are its verbs instead of Edits.

**Pickup** — an item entering a Player's inventory. Not built; distinct from Use
and from Sample, which copies what is aimed at into the hand.

**Measurement** — two corners a Tool's Use resolved, kept by the Client for the
session. A corner is a vertex of the struck face of a Voxel's drawn Shape.

**Command** — a Mod's request to fill a region or place Voxels at positions, to
place or remove sources in a region, or to ignite or extinguish what is in one.

**Command batch** — the Commands returned together by one Mod call.

**Job** — one Chunk's worth of one streaming stage (generate, solve light,
mesh), run on a worker from a Snapshot and returning an owned result. A Job
never touches World state.

**Snapshot** — the immutable view of World state a Job is handed: the Chunks,
their light and their overlay, as shared references.

**Landing** — the Server applying a finished Job's result to World state, on its
tick. The only way a Job's work becomes part of the World.

## Simulation

**Overlay** — the fluid and fire a Voxel holds beside its packed Material, Shape
and Orientation: a second, palette-compressed layer of each Chunk. A dry,
unburning Chunk's is uniformly empty.

**Occupants** — the one reading of a position: its solid, its fluid and whether
it burns, decoded from the Voxel and its Overlay together. Nothing interprets
either layer alone.

**Fluid** — a Material that flows, with a row of behaviour in metres and
seconds: reach, sideways, downward and recession rates, reset drop, whether it
waterlogs or extinguishes, and its heat. None is built in: water is a Game's
Material like lava, so a World with no Game has no fluid. A Voxel holds at most
one fluid Material.

**Source** — fluid that stays: generated lake fluid, and fluid a player or a Mod
places. It feeds flow without losing volume, and only an Edit, a Command or a
reaction removes it. Flow never becomes one.

**Flowing** — fluid a source supports, carrying its remaining reach and its
descent since the reach was last renewed. It recedes when nothing supports it.

**Logical step** — one step of simulation time, 1/64 s. Equal state and equal
events at equal steps give equal state, whatever the workers or the wall clock
do.

**Lifetime** — which arrival of a Chunk at its coordinate a piece of work read.
An evicted and regenerated Chunk has a new one, so work that read the old one
cannot land in it.

**Agenda** — the Voxels due for evaluation and the step each is due, one entry
per Voxel, with a fixed capacity.

**Rescan** — a box of a Chunk evaluated a budget of Voxels a step from a cursor:
what a wake becomes when the Agenda is full, and how admission and the first
scan of a wet Chunk are served.

**Parked** — work held back for a Chunk whose light solve is pending, released
as one Rescan when the solve lands.

**Admission** — a Chunk joining the simulation, when its first light solve
lands. Until then its terrain is unknown, never Air.

**Void** — a connected region of empty sixteenths in a Shape that reaches one of
its faces, derived when the Shape is registered. Fluid stands in, flows through
and passes heat through Voids; a sealed cavity is not one.

**Waterlogging** — fluid standing in a Shape's Voids. The Shape keeps its
Material, collision and selection.

**Contact** — two different fluids whose wet Voids meet across a face, where
flow could pass between them.

**Reaction** — what a Contact between two fluids does, declared as data: which
side is consumed and the solid, if any, it leaves.

**Receiving** — of two fluids in Contact, the one the other arrives at: from
above, or from beside on a floor no lower, never from below. A Reaction may name
it.

**Blocked contact** — a Contact whose Reaction cannot apply as a whole, because
its product would need a Voxel a solid holds. It waits until something it reads
changes.

**Exposure** — heat reaching fuel through open space, from 0 to 15:
dimensionless, from the strongest hot fluid or fire within its reach in metres.
Solids block it, and nothing stores it.

**Ignition progress** — how long fuel has been exposed at or above its
threshold, towards its ignition delay.

**Burning** — a solid on fire, with the fuel it has left. A whole Voxel burns,
whatever its Shape.

**Burn-out** — a fire running out of fuel: the solid becomes its declared
residue in the same Shape and Orientation, or goes if the residue is Air.

**Flame indicator** — the placeholder flame drawn just outside each uncovered
face of a Burning solid. It is not a Voxel and changes no collision.

**Wet fuel** — fuel an extinguishing fluid touches, in one of its own Voids or
from a neighbour's Void across a face its solid covers. It does not ignite, and
it stops burning. A wet part makes the whole Voxel wet.

**Coarse** — a level-1 Chunk with its Overlay, derived from the held level-0
Chunks below it.

**Child** — one of the eight finer Chunks a Coarse Chunk is derived from:
generated, or held and changed and read with its Overlay.

**Waterline** — the height below which packed lake fluid at level 1 is a lake
generation could have made; any other fluid there is dynamic.

**Emitter bound** — how many changes that start or stop block light, such as
lava arriving or fuel catching, one Logical step may land (`EMITTERS_PER_STEP`).
Each relights the whole reach of its light.
