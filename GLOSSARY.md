# Tesserae — glossary

Canonical vocabulary. Terms here are the only names used in code, docs and the
mod API. This file is a glossary: no decisions, no implementation.

See [DESIGN.md](DESIGN.md) for decisions and [docs/adr/](docs/adr/) for the ones
that are hard to reverse.

## Content

**Engine** — Tesserae itself. Ships no gameplay.

**Game** — a Mod whose manifest declares it a root. There is one loader and one
format; a game is not a separate kind of thing. Several are installed side by
side under `games/`; a World uses exactly one.

**Mod** — a unit of content loaded by the engine. Declares block behaviour,
entities, recipes, worldgen parameters, assets. Installed under `mods/`; a World
freezes the set chosen at Create, dependencies included.

**Data-only Mod** — a Mod whose Declaration is supplied as data, with no guest
code. It may be the World's Game or add content alongside other Mods.

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

**Commit** — one immutable, checksummed record of a World's authoritative state
at a moment. Opening a World opens its newest complete Commit. Not "snapshot",
which is a Job's view.

**World list** — the launcher's list of every World folder, reached from the
title screen before any Game runs. Not the Menu, which opens inside a World.

**World session** — one opening of a World, from loading to Return to Worlds.
The window and the app outlive it; everything it added to the ECS, its jobs and
its Mod instances do not.

**Voxel** — the atomic unit of world volume. Not "block", not "node".

**Material** — what a Voxel is made of. Stone, water, oak. A row of game data:
the light it emits and the light it dims, which faces it hides, which mesh it is
drawn in, and whether it is solid. Not its appearance: colours and textures are
separate Declaration data, shared by World surfaces and item previews until a
Look pack supplies appearance. Bevy's `Material` and `MaterialExtension`, which
shade a surface, are a different thing.

**Tag** (of a Material) — a label a Material carries, such as `soil`, `sand` or
`water`, that a Plant's rows can name as `#tag` in place of one Material. Not a
Station's tag, which a Recipe names.

**Shape** — a Voxel's geometry. A full cube, a stair, a slab, a fence. "A stone
voxel" and "a stone stair" share a Material and differ in Shape, so neither word
alone identifies a Voxel.

**Orientation** — a Shape's rotation: one of the 24 axis-aligned rotations.
Meaningless for a full cube, which looks the same under all of them.

**Nodebox** — an axis-aligned box inside one Voxel, in sixteenths of its edge. A
Shape is a union of them.

**Flat form** — how a Shape is drawn when it is a plant: **Cross**, two diagonal
quads through the Voxel's centre, or **Plane**, one quad against a face. Its
Nodeboxes still decide selection and where fluid can stand. It is walked through
unless the Shape declares collision, seals no face, cannot connect and does not
count against a Chunk's shaped Voxels. Every other Shape is drawn from its
Nodeboxes.

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

**Sea level** — the height in metres of the ocean's surface. Ground below it,
away from land, is the sea floor. World generation fills the open air under it
with the Game's water fluid, any fluid it declared; a Game that names none, and
a World with no Game, have no sea.

**Water surface** — the height in metres of standing or running water at a
column, if there is any: the sea at Sea level, a lake at its own flat level, a
river stepping down its course. One answer per column that Settlements, bridges,
the Waterline, coarse levels and spawn all ask. A Game that declares no fluid
still has the basins, dry.

**Landform** — the large-scale shape of the ground (ocean, coast, plain, hills,
plateau, canyon, mountain range) that fields of the seed and a position in
metres lay down, before Biomes, water or Caves. Engine-owned: a Game tunes
landforms and cannot add a kind.

**Light level** — how much light reaches a Voxel, from 0 to 15. The one quantity
in the engine that is **dimensionless**: a count of attenuation steps, not a
distance, so it does not scale with Voxel size the way everything physical does.

**Sky light** — the light a Voxel receives from the sky. At full strength it
falls straight down undimmed, so open ground is lit the same however tall the
air column above it is. A face takes a share of it by which way it faces, as a
wall sees less of the sky than a top: all of it on a top, less on a side, least
on an underside.

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
distance from the Eye, in metres. Its outer distance is set by the Quality
curve.

**Quality curve** — what sets the outer distance of every coarse LOD level's
ring, from two values in metres: a unit `u` and a reach, so coarse level k ends
at `u * 2^k` or at the reach, whichever is nearer. A Player's preference, which
the Server clamps to the Game's Ceiling and streams from; never derived from a
screen's size. Level 0 ends at the Game's view distance under every curve.

**Ceiling** — the farthest a Game lets any Quality curve reach: its declared
`lod-distance-m`. The Server clamps every request to it. _Avoid_: max distance

**Impostor** — a Tree or Structure drawn at a coarse LOD level from the same
pure draw that stands it at level 0: in the same place, in coarse Voxels, kept
where the half-volume vote would lose it. Never stored and never edited.
_Avoid_: billboard, proxy

**Canopy cover** — what a coarse Voxel draws where its Voxel is too large for
single trees to read: the share of its column that level 0's trees would cover,
from the same Biome and density fields, shown as a canopy rather than as
Impostors.

**Coarse agreement** — a coarse column answers water or land, and its top
Material, as level 0 does at that column's centre in metres.

**Distance fog** — how much of a surface's colour is replaced by the fog colour,
as a function of its distance from the Eye in metres. Two parts: a haze in
metres that fades blue first and stops short of hiding anything, so 100 m reads
farther off than 2 m at any reach; and a fade sized from the full-detail radius
and the reach, complete at the reach. Neither takes a Voxel size, so it is the
same fog at any Voxel size. The fog colour is the sky's at the horizon; the
sky's own colour is its zenith.

**Downsample** — producing a coarse Chunk's Voxels from the eight finer Chunks
it covers: the dominant Material by volume, solid if half the volume is. At
level 1 fluid weighs the volume it fills. From level 3 up only where an Edit or
saved change reaches; elsewhere those levels are generated at their own Voxel
size, one height per column (ADR-0011).

**World generation** — producing a Chunk's contents from a seed and a position.
Never produces a mesh.

**Climate** — continuous fields, temperature and humidity, that are a pure
function of the seed and a position in metres. They choose the Biome; they are
not a Biome. Temperature falls with altitude and humidity rises toward the sea,
and Weather reads the same Climate.

**Biome** — a region type a Game or Mod declares and Climate selects. It owns
the ground Materials, surface features, ores, cave tuning and the sky and fog
colours of the columns it covers. It is also selected by altitude, distance from
the coast and erosion, and may scale its landform's detail relief; a Biome that
names no range matches every value on that axis. A cave Biome is chosen the same
way in three dimensions.

**Surface feature** — a Material a Biome declares and world generation stands
one Voxel above that Biome's full surface Voxels, from the seed, at a mean
spacing in metres. Level 0 only; a Mod still places no terrain.

**Clump** — an optional scale in metres and strength on a surface feature: a
low-frequency density field of position that multiplies the feature's chance, so
it stands in groves with clearings between while its declared spacing stays the
mean spacing.

**Field** — an optional group, region scale and strays on a surface feature: the
ground is cut into regions, each given one member of the group, so a patch is
mostly one species with a few strays.

**Crop field** — a Biome's planted field: a grid of rectangular plots, each one
crop in rows, with a path between them and a ditch along the low edge, standing
on level ground clear of the lake, caves and trees.

**Generator version** — the revision of the world generation algorithm a World
was created under. A World keeps it for life, so a later revision never changes
its terrain for the same seed. Version 3 is the only one the Engine generates; a
World stamped 1 or 2, from before the first release, refuses to open.

**Stratum** — a layer of a Biome's rock between two depths below the surface,
its edges undulating a few metres.

**Cave Biome** — a region of the underground, chosen in three dimensions from 12
m under the surface, with its own rock, cavern floor, features and fog.

**Aquifer** — a region of the underground whose water table stands above the
World's, so its cavern lakes stand higher and level across it.

**Spring** — a one-Voxel pool welling up in an aquifer's dry cavern floor,
sealed by rock below and beside, so it stands rather than pours.

**Outskirts** — a Settlement's Buildings past its disc, beside its approach
paths and round its edge, in groups each laid by its own chance: farms, ponds,
brooks and waterfalls.

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

**Use** — a Player pressing Use: a held Tool's action; with no Tool in hand,
using the Usable Voxel aimed at within Edit reach; otherwise eating the food in
the selected Hotbar slot. Answered by the Server, never applied: it enters no
history, and changes no Voxel except a Door it toggles. Not an Edit, not Sample,
and not Pickup.

**Tool** — a thing a Player holds whose Use the Server runs, declared by the
Game by name and kind. A hand state taken with its own key, not from the
Catalogue: while one is held, Place and Remove are its verbs instead of Edits.

**Item** — a thing a Game or Mod declares by name, with a size in metres. Not a
Voxel and not a Tool. A Material may be tied to one Item: removing a Voxel of it
drops that Item. A placeable Material no declared Item is tied to has an
implicit Item of its own name.

**Entity** — a Server-owned thing with a position in metres that is not a Voxel,
fluid or Player. Not a Bevy `Entity`.

**Dropped item** — an Entity holding a count of one Item, falling or at rest,
until picked up or despawned.

**Inventory** — the Server-owned Stacks a Player holds: a 27-slot backpack and a
9-slot Hotbar. Replaces Carried.

**Stack** — one Item and a count, at most the Item's stack limit. Shape and
Orientation are the Player's placement choice, not part of a Stack.

**Hotbar** — the nine Inventory slots usable in play; one is selected.

**Catalogue** — the Creative-only list of Materials a Player can take Items
from. Not the Inventory and not ownership.

**Carried** — retired: what a Player held after Pickup before the Inventory. Use
Inventory.

**Pickup** — a Dropped item entering a Player's Inventory, by nearness. Distinct
from Use and from Sample, which copies what is aimed at into the hand.

**Recipe** — the Items spent and the Item made by one Craft, and where it is
crafted: by hand or at a Station. Declared by a Game or Mod.

**Craft** — a Player making a Recipe's Item: the Server spends the inputs and
gives the output together. Not an Edit.

**Usable Voxel** — a Voxel whose Material is declared a Door, a Container or a
Station, so Use acts on it.

**Station** — a Material whose Use opens the Recipes that name its tag, crafted
only while the Player stays within Edit reach of it. Materials may share a tag.
A hand Recipe needs no Station and is crafted from the Inventory anywhere.

**Container** — a Material whose Voxel holds Stacks of its own, apart from any
Player's Inventory. A Player cannot remove one that holds Stacks; anything else
that removes it spills them as Dropped items.

**Door** — two Materials, closed and open, that Use toggles between, and a size
in metres, 1 m by 2 m unless declared, so it is as large at every Voxel size.
One Item places the whole Door, Use toggles all of it, and removing any Voxel of
it removes it all and drops the closed one's Item. Its row may fix a Shape, as
Terra's fixes a thin panel: placing turns the panel to face the Player, hinged
on the half of the cell aimed at, and Use turns each Voxel of it a quarter about
that hinge edge. Open, only a leaf's hinge edge shows its panel: the rest folds
into it, drawn as nothing and passed by aim, so a leaf opens as one panel
against its jamb. Bodies collide with the panel as drawn, except that an open
Door with no fixed Shape, or wider than one Voxel, lets them through.

**Game mode** — Creative or Survival. A World has a default; each Player has
their own. Both spend an Item per placed Voxel and drop one per removed Voxel, a
whole Door counting as one; Creative's Catalogue gives Items, and only Survival
has damage, hunger and death.

**Vitals** — a Player's health, hunger and air.

**Measurement** — two corners a Tool's Use resolved, kept by the Client for the
session. A corner is a vertex of the struck face of a Voxel's drawn Shape.

**Layout** — a line, rectangle or box of Voxels a Player places with one intent:
a count per axis, a gap in Voxels and a direction from an anchor. The Server
expands it, judges each cell as it would a single Place, and records what it
placed as one undo group. Not a Command, which is a Mod's request.

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
reaction removes it. Flow never becomes one. Removing one drops its Item; no
Edit aims at flow, so it drops nothing.

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

## Plants

**Plant** — a row of data that the Server grows: one of five Kinds, a Seed, what
it needs and what it gives. The Engine ships none; a Game or Mod declares them.

**Kind** (of a Plant) — the closed set of behaviours a Plant row names: `crop`,
`tree`, `column`, `spreader` or `climber`. A **crop** passes through Stages in
one Voxel; a **tree** grows from a sapling into a trunk and canopy; a **column**
adds one Voxel at a time up or down (cane, kelp); a **spreader** copies itself
onto a neighbouring Voxel (grass, mushrooms); a **climber** grows along the
faces of solids (a vine).

**Stage** — one Material a Plant passes through. A Plant's state is its current
Stage, so nothing is stored per plant. The last Stage of a crop is ripe. A Stage
may name a Shape of its own, which its Voxels take.

**Grown Material** — a Material only growth and generation make: a tree's trunk,
canopy and fruit, and a Plant's later Stages. It has no Item and cannot be
placed. What a Player places, however alike in look, is not one. A Template may
hold a crop's later Stage, a field planted earlier, but no tree's.

**Seed** — the Item that places a Plant's first Stage: a seed, a sapling or a
cutting.

**Fruit** — the grown Materials some canopy Voxels of a tree ripen through.
Breaking one drops its yield (a ripe one) and leaves canopy; a standing tree
grows new fruit elsewhere on its canopy over time.

**Vine crop** — a crop whose ripe Stage, the vine, bears fruit on the ground
within reach, never touching another and up to a cap; the fruit never seeds, and
breaking one lets the vine bear another.

**Min-neighbours** — a spreader's floor: it seeds a target only where that many
of its kind stand within reach, so a patch fills its holes and does not advance.

**Branching** — the optional record on a tree row that gives it a lean, a fork
into leaders, limbs and lobes, in metres and fractions of its height. A tree
with none is a straight trunk under one crown.

**Limb** — a stepped one-Voxel line of a tree's trunk Material running out from
the trunk or a leader, so it is felled and cut like any trunk Voxel. A
**leader** is one of the two or three limbs a fork splits the trunk into.

**Lobe** — an ellipsoid of a tree's canopy Material around a limb or leader tip;
the crown on the trunk's top is the largest lobe.

**Storey** — one height band a tree row may declare: a share of its trees, their
trunk height and their crown's size. Each tree belongs to one, fixed by the
World seed and where it stands, so a forest has trees above and below its
canopy. Where crowns meet, the taller storey's Voxel wins. A tree row with no
storeys is one band.

**Canopy variant** — one of several canopy Materials a tree row may declare,
alike but for colour. Each tree grows one, fixed by the World seed and where it
stands, so neighbouring crowns read as separate trees.

**Vines** (generated) — climbers that worldgen hangs from a tree's canopy. A
growing sapling never makes them.

**Sample** — the positions a held Chunk draws each Logical step, and the roll
each makes, from the World seed. A Plant Voxel that is Sampled and passes its
roll may grow.

**Growth** — a Plant Voxel changing Material, or a tree, column, spread or vine
adding Voxels, because a Sample's roll passed and the Plant's needs held. A
Plant that fails a need holds its Stage and never dies of it.

**Felling** — the breaking of the grown Voxels of one tree that no longer reach
a grounded trunk, top-down over several Logical steps, after a trunk Voxel is
cut. Placed Voxels are never part of one.

## Map

**Map** — what the Server holds for each Player of the surface they have
explored: per column sample, the surface height, the top Material and the Biome
as they were when seen. Not a view of the World and never regenerated from the
seed, so it shows a Player's builds and lakes as of their last visit.

**Map tile** — a fixed square of a Map, in metres, the unit the Server sends the
Client and the Client caches. Not a Chunk, which is data of Voxels, and not a
Cell.

**Explored** — a column sample a Player has been near enough to see while its
Chunk was held and lit. Terrain that was not loaded is never Explored, so a Map
never guesses at it.

**Waypoint** — a named, coloured point a Player places on their own Map. The
Server also adds one where a Player died.

**Marker kind** — the name, icon and colour a Game declares for a Waypoint. A
World with no Game has none.

**Compass** — the Client's strip showing the Player's heading, the four cardinal
directions and the bearing of each Waypoint. Presentation only. Not the Map.

**North** — the World's −Z axis, in every World at every Voxel size. The
direction the Player faces at heading zero.

## Structures

**Template** — a named block of Voxels a Game or Mod declares: each Voxel is a
Material, a Shape and an Orientation, laid out on a grid whose size is given in
metres. Data, not a place in a World; a World with no Game has none.

**Template cell** — one square of a Template's grid, `cell-m` on a side, in
metres: Keep, Air, ground or a Voxel. Not a Cell, which is a Chunk's footprint.

**Keep** — a Template cell that is never written, so the ground a Structure
stands in shows through. Air in a Template is not Keep: it carves.

**Ground layer** — the layer of a Template that sits at the ground's height
where a Structure is placed. Layers below it are underground, such as a cellar.

**Obtainable** — of a Material, that a Player in a World can get its Item: from
what world generation lays, grows or drops, or by Recipes from things already
obtainable, at a Station that is itself obtainable. Structures and the Catalogue
are not sources. Every cell of a Template must be obtainable in each World it is
used in.

**Structure** — a Template placed in a World, by world generation, by a Player's
Tool or by a Site. Rotated about the vertical axis in quarter turns, and the
same size in metres at every Voxel size. Nothing is stored per Structure: once
placed it is only Voxels.

**Settlement** — a group of Structures that world generation lays around a
centre and joins with paths, from rules a Game or Mod declares: a radius in
metres, an optional centre Template, the Templates it may hold and a path
Material. Its Plots and radius may be a range, and each copy draws its size from
it. A hamlet, a village and a town are a Game's names for Settlements of
different rules, not different things.

**Lattice cell** — one square of a lattice world generation spreads Structures,
Settlements or crop fields on, a row's spacing in metres on a side. It holds at
most one candidate. Not a Cell.

**Building** — a Template a Settlement stands at its centre or on a Plot, with
the Template cell of its door if it has one.

**Plot** — the ground one Structure of a Settlement stands on. A Plot whose
ground fails the rules stays empty.

**Apron** — the 2 m band round a Plot's footprint that world generation eases
from the Plot's level back to the natural ground, so a levelled Plot has no
cliff at its edge.

**Road** — a Settlement's way between its centre and its Plots, laid by world
generation in the Settlement's path Material on graded ground, as stairs and
slabs where it climbs. Every Plot's door faces one. Not a Layout and not a crop
field's path.

**Loot** — the Items a chest world generation stands holds, drawn by weight from
a table a Settlement, one of its Buildings or a Biome declares, the first time a
Container is needed for it. A chest a Player places, or the place Tool stands,
holds none. Not a drop, which a broken Voxel gives.

**Blueprint** — an Item that names one Template. Laying it makes a Site.

**Site** — a laid Blueprint: a Structure not yet built, kept by the World until
its last Voxel is placed from Items brought to it. Once complete it is only
Voxels, like any Structure.

## Audio

**Sound** — a thing a Game or Mod declares by name that can be heard: one or
more recordings, one picked each time, with how loud it is, how far it carries
in metres and its Sound category. None is built in, so a World with no Game is
silent.

**Sound event** — the Server making one Sound heard at a position in metres,
because of a Cause. The Client plays it and never decides that it happens.

**Cause** — why the Server makes a Sound event: a Voxel broken, placed, struck
or stepped on, a landing, a splash, swimming, floating, a Pickup, a Use
answered, food eaten, a Player hurt or dying. A closed list the engine owns;
what each one sounds like is the Game's.

**Sound set** — the Sounds one or more Materials make for the Voxel Causes:
break, place, hit, step and land. A Material with none is silent for them.

**Sound category** — the group a Sound's loudness is set by in a user's
settings: Effects, Ambient or Music, all under Master. A user's preference, not
World state.

**Soundscape** — what a Player hears around them when nothing happens: looping
ambient Sounds and a list of music. A Game declares one per context (default,
underground or underwater) and a Biome may name its own; the Server says which
applies to the Eye.
