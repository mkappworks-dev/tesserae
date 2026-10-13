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

## World

**World** — one save. Owns its Voxel size, its seed and its Chunks.

**Voxel** — the atomic unit of world volume. Not "block", not "node".

**Material** — what a Voxel is made of. Stone, water, oak.

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

**Air** — the absence of Material. A Voxel, not a null.

**Cave** — a tunnel of Air that World generation carves through solid ground.
Sized in metres, so it is the same Cave at any Voxel size. Where one breaks the
surface it has a **mouth**, and sky light can enter there.

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

**Downsample** — producing a coarse Chunk's Voxels from the eight finer Chunks
it covers: the dominant Material by volume, solid if half the volume is.

**World generation** — producing a Chunk's contents from a seed and a position.
Never produces a mesh.

## Runtime

**Server** — the authority over World state. The only thing permitted to mutate
a Chunk.

**Client** — renders and sends intent. Never mutates World state.

**Integrated server** — a Server running in the Client's process, for single
player. Not a different Server.

**Player** — what the Server holds for each person in the World, including where
that person is. The Client reports a position; the Player is the Server's idea
of it.

**Eye** — the point in metres a Player sees and acts from. An Edit's ray starts
near it. Not the feet and not a body.

**Edit** — a change to one Voxel, applied by the Server. A Client sends an
intent; it never makes an Edit.

**Job** — one Chunk's worth of one streaming stage (generate, solve light,
mesh), run on a worker from a Snapshot and returning an owned result. A Job
never touches World state.

**Snapshot** — the immutable view of World state a Job is handed: the Chunks and
light it reads, as shared references.

**Landing** — the Server applying a finished Job's result to World state, on its
tick. The only way a Job's work becomes part of the World.
