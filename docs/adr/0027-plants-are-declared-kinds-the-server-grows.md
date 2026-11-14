# Plants are declared kinds the Server grows

Status: accepted.

Step 18 makes vegetation live. A Game or Mod declares **Plants** as data, each
with a closed kind (`crop`, `tree`, `column`, `spreader` or `climber`), and the
Server grows, harvests and fells them in native code. No Mod is called when a
Plant grows or is broken ([ADR-0001](0001-wasm-mod-boundary.md)); a new kind
needs new engine mechanism and arrives with the contract line that adds it. The
Engine ships no Plant, so a World with no Game has none
([ADR-0016](0016-only-air-is-built-in.md)).

## A Plant's state is its Material

A crop's **Stage** is a Material. A sapling, a trunk, a canopy and a ripe fruit
are Materials. Growth is the Server changing one Voxel's Material through the
one World change, so saving, meshing, light, collision and the Client's drawing
of an edit need nothing new. Nothing is stored per plant: the Overlay, the
Agenda and the save format are unchanged and no Generator version moves. A World
saved with a Plant whose row has since gone still loads; its Voxels are
Materials like any other, and they stop growing.

A Material is at most one Stage of one Plant, once. A Stage after the first, a
tree's trunk and canopy, and fruit get no implicit Item and cannot be placed or
taken from the Catalogue, as a Door's open Material cannot
([ADR-0026](0026-usable-voxels-are-declared-kinds-the-server-runs.md)). The
Plant's **Seed** is an Item whose Material is its first Stage, and placing it is
Place from the selected slot like any Item.

## Growth is a Chunk-sampled tick

Growth runs on the Logical step
([ADR-0014](0014-simulation-advances-in-bounded-logical-steps.md)) as a
**Sample** tick, in the clock of the fluid simulation and through the same
`set_occupants` batch:

- Each step, every held level-0 Chunk whose palette holds a Plant Material draws
  `PLANT_SAMPLES` (32) positions. Where they fall is a pure function of the
  World seed, the Chunk, the step and the index, and so is each position's roll,
  so equal inputs give equal draws at every worker count.
- A drawn Voxel that is a Plant Stage passes its roll with probability
  `min(1, interval ÷ mean-seconds)`. The `interval` is the mean time between
  Samples of one Voxel, `CHUNK_VOLUME ÷ (PLANT_SAMPLES × 64)`, 16 s. A
  `mean-seconds` below it is a load error, so 16 s is the documented ceiling on
  growth speed and a Stage takes about its `mean-seconds` whatever the Voxel
  size, in seconds and metres.
- A passing roll checks the Plant's needs against the live store: soil, light at
  the Voxel, Climate at the column in metres, and water within reach. A failing
  need leaves the Voxel as it is, so a Plant never dies of a failing need, and a
  farm left in the dark waits. A Chunk the simulation has not admitted, or one
  with a light solve pending, is read as failing, never as Air.
- Changes land under `PLANTS_LANDED_PER_STEP` a step and, for whole trees,
  `TREES_PER_STEP`, apart from it. One past a bound waits for the next step with
  its roll kept, and is dropped if its Voxel or what it read has changed.
- Only held Chunks Sample. A Chunk evicted and reopened resumes as it was and is
  never credited the time it was gone
  ([ADR-0013](0013-fluid-and-fire-live-beside-the-voxel.md)). There is no
  catch-up.

The Sample tick runs inline at the end of each Logical step, after the step's
fluid landings and before the step advances, so what it reads cannot be stale
within the step. The spec had it read a worker Snapshot; the code reads the live
store, which makes the stale-read rule above a rule about deferred changes only.

The benchmark pass measured the four numeric constants and kept three:
`PLANT_SAMPLES` 32, `PLANTS_LANDED_PER_STEP` 64 and `FELLED_PER_STEP` 32.
`TREES_PER_STEP` is 1, because an oak at 0.25 m is 3341 Voxels and 6.1 ms of CPU
to land, 40% of a 15.6 ms step. A bound counted in trees is a blunt one when a
tree's cost grows with the cube of the Voxel size's inverse; a bound in Voxels
would fit better, and waits for a World that needs it.

**Kinds.** A `crop` is one Voxel through 2 to 8 ordered Stages, the last ripe,
with an optional `harvest-to` Stage a Remove of the ripe one resets to. A `tree`
is a sapling that grows a parametric tree as one all-or-nothing group. A
`column` adds one Voxel at the end of its run, up or down, to a height in
metres, optionally standing in a fluid (kelp). A `spreader` copies itself onto a
hash-picked Voxel within `reach-m` that meets its needs, while a `density` cap
holds. A `climber` adds a Voxel on an adjacent face of a solid or hangs below an
overhang, up to `length-m`. Every size and rate is in metres and seconds and is
converted when the World opens ([ADR-0003](0003-voxel-size-is-per-world.md)).

**Support.** A plant Voxel needs soil below it (above for a column growing down,
in any neighbour for a climber, on the face its Orientation names for a vine).
When a Voxel next to a plant changes, the Server's existing wake-on-change
re-reads its support, and a failing Voxel breaks and drops its Seed in the same
Logical step. Nothing is polled. Light, Climate and water are growth needs and
never placement rules; only `soil` refuses a Seed ("Wheat needs soil.").

**Soil and water are tags.** A Material may carry up to 8 `tags`, and a Plant's
`soil`, a column's `stands-in` and a water `fluid` may name `#tag`, which
expands after every Mod has declared to each Material that carries it (solid
ones for soil, fluids for water). A tag that matches none is a load error naming
the Plant. A shared Mod can say `#soil` and grow in any Game whose ground
carries it.

**Plants carry their Shape.** A Plant may name the Shape of its single-Voxel
Stages. Placing the Seed, growth and generation all use it, whatever the Player
chose, and a growing Voxel copies the Shape and Orientation of the Voxel that
grew it. For a `plane` the Orientation is the one that holds the face the Player
struck. Fruit carries its own Shape, so fruit is a small 3D model of boxes
rather than a cube or a flat quad.

**A Stage may carry its own Shape.** A `stage` has an optional `shape`; none
keeps the Plant's (or the fruit's). When a crop or fruit advances, or a ripe
`harvest-to` crop resets, the Voxel takes the new Stage's Shape and keeps its
Orientation, so a berry bush is a stem, a young mound and a mound with berries,
and unripe fruit is a smaller Shape than ripe fruit. A seed places its first
Stage's Shape, a fruit that re-bears is Stage 0 in Stage 0's Shape, and worldgen
draws fruit at its hash-drawn Stage in that Stage's Shape, so no Voxel is
generated in a Shape its Stage would not grow into. A Biome freezes the Shapes
by Stage and name as an optional field, absent in every snapshot written before
it. Undo and redo still read every Stage as one Voxel, Shape included: the
journal compares a Voxel with its chain's first Stage in that Stage's Shape.
Spreaders, columns and climbers are one Material each and need none.

## Trees: grown Materials against placed look-alikes

A tree's trunk and canopy are **grown Materials**, made only by growth and
worldgen. A Game declares a placed Wood and a grown trunk of the same look, and
a Player builds only with the placed one. Felling tells a grown tree from a
build by identity-free Voxels: no Voxel records which tree it belongs to. When a
grown trunk Voxel changes, the Server evaluates the **Felling** region: the
grown Voxels of that Plant's trunk, canopy and fruit Materials, connected
through faces, that no longer reach a trunk Voxel standing on something that is
neither Air nor a grown Material, so a placed Voxel, a sapling or a crop below
holds the tree up. Only grown Materials take part, so a cabin of placed logs and
leaves, however alike in look, is never in a region and never falls. A region
wider than `2 × reach + 3` Voxels (the tree's reach, below, plus a few Voxels of
rounding) or taller than the tallest trunk plus the larger of `canopy-radius`
and the lobe radius, `+ 3`, or one beside a Voxel the Server does not hold, does
not break.

A region breaks top-down over successive steps, `FELLED_PER_STEP` (32) Voxels a
step, each dropping its declared yield. A Felling also drops `sapling-yield`
Seeds, at least one, once for the tree when the region is first queued, so every
tree cut down can be replanted; a leaf cut that leaves the tree grounded drops
none. Player Remove, Mod Commands and burn-out all reach plant Voxels through
the one write path, so all of them can start a Felling. Falling as a moving body
is step 24's.

A tree is parametric until Structures (step 20): a hash of seed and position
chooses a whole trunk height inside `trunk-height-m`, and the canopy is an
ellipsoid of `canopy-radius-m` centred on the trunk top. A tree is a pure
function of the seed and a column in metres, so a Chunk generates the trunk and
canopy Voxels of every neighbouring column that fall inside it, and a tree
crosses Chunk borders whole and is the same at every Voxel size. A sapling grows
only if every cell of the group is Air or foliage, and otherwise stays a sapling
and Samples again. Foliage is what the tree overwrites: its own grown Materials,
ground cover and small plants (a spreader, climber or column), any tree's leaves
and fruit, and a sapling. A trunk of another tree, a crop, a placed Voxel,
terrain, a fluid and fire block. A dense understory and a neighbour's crown
would otherwise hold a large tree's sapling for ever, since a jungle's vines and
ferns fill its cells at generation. A species' Climate band must cover its
Biome's whole region, not only the Biome's point, because Climate is a
continuous field and a Biome is the nearest point.

Fruit replaces canopy Voxels chosen by a hash of seed, position and `density`
(at most 0.3). A fruit Voxel ripens through its grown Stages like a crop, with
the tree's light and Climate, light read as the brightest of the Voxel and its
six neighbours since fruit sits in canopy. Fruit is harvested by breaking it
(below), never by Use, and a standing tree grows new fruit elsewhere on its
canopy.

**Re-bearing.** A canopy Voxel of a tree that declares `fruit` is a re-bearing
site, a Sample of it rolling like any other Stage. Nothing is stored per tree. A
tree's expected canopy cell count `N` is the mean number of leaf and fruit cells
of four reference trees (`tree_voxels` at fixed bases) at the World's Voxel
size, a pure function of the row cached per Plant. Each of its `N` canopy Voxels
has a mean of `N * regrow-seconds`, so the tree as a whole rolls one passing
Sample about every `regrow-seconds` whatever the Voxel size. A passing roll
turns the Voxel into the fruit's first Stage, with the fruit's Shape, when the
tree's light and Climate hold (its `needs` minus soil) and the fruit within
twice `reach-m` of it, which is a tree's span, is below `density * N`, the share
the tree is generated with. A felled tree has no canopy left to Sample, and a
Voxel queued for Felling never bears. A tree of the same species closer than
that span shares the cap, which only holds fruit back. A `regrow-seconds` is at
least the 16 s Sample interval; a Sample costs nothing on a Chunk with no Plant
Material.

A small plant model (fruit, a berry bush, a mushroom) is a `boxes` Shape whose
Material's tile is painted by position: a box face samples the tile at the
face's own place in the Voxel, so one tile gives each box its colour. Such a
Shape costs one against the Chunk's shaped-Voxel cap, which generated Terra
stays far under (at most 292 of 1024 in a 9,000 Chunk sample, 106 of them
berries and mushrooms).

**Branching** is an optional record on a `tree` row, all in metres or fractions
of the trunk height so a tree is the same at every Voxel size: `limbs` (a count
range), `start` (the fraction of the height where limbs begin), `reach-m` (a
limb's horizontal length), `rise` (-1 drooping to 1 ascending), an optional
`fork` (the height fraction where the trunk splits into two or three leaders),
`lean-m` (how far the top stands from the foot), `tiers` (limbs in rings of four
that shorten towards the top, a cone) and `lobe-radius-m`. Limbs, leaders and a
leaning trunk are **trunk Voxels** of the tree's grown trunk Material, a
face-connected stepped line one Voxel thick, so Felling needs no new rule: a cut
trunk fells everything that no longer reaches the ground, limbs and lobes
included, and a cut limb drops its end and what hung on it while the trunk
stands. Foliage is a **lobe**, an ellipsoid of the canopy Material, at every
limb and leader tip (and along a tier's limb), besides the crown of
`canopy-radius-m` on the trunk's top; fruit and worldgen vines treat lobes as
canopy. Every draw is a hash of seed, base position in millimetres and the row;
directions are one of sixteen literal compass vectors, so no platform's
trigonometry moves a limb. A tree with no `branching` generates exactly as
before. Its **reach** is the lean plus the larger of the crown radius and a
fork's spread (half of `reach-m`) plus the limb plus the lobe, and must stay
within the 8 m `canopy-radius-m` bound, so worldgen's per-Chunk scan bound does
not grow; the fork's spread must also stay inside the crown and lobes, so the
tree is one piece. A branched Biome tree freezes its record in the saved
generation data as an optional field, so a Game that later edits its row cannot
reshape a saved World.

**Vines are generated only in worldgen.** A tree row may name a `climber` and a
density and length. Worldgen hangs it from the outer canopy by a pure function
of the tree, seed and Voxel size, whole across Chunk borders, and a growing
sapling never does. Once placed, vines live by the climber's rules.

## Harvest, undo and what is not journalled

Remove of a plant Voxel drops that Stage's declared yield (but see the harvest
rule below), with each count a hash of seed, position and Stage inside its
range, never a draw from a generator. A Stage that declares none drops the Item
that places it. Creative drops the same. A removal's drops ride the undo entry
as `Cost::Yielded`
([ADR-0023](0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md)):
undo takes back what still lies there and spends the rest from the Inventory,
refusing whole without it.

- **A crop's Stages are one Voxel to undo and redo**, as a Door's two Materials
  are, so undoing a Remove or a Seed placement works after the crop grew.
- **A tree is the exception.** A trunk is not in the sapling's chain, so undoing
  the Seed's placement after the tree grew is refused by the journal's ordinary
  mismatch rule and never half-removes a tree.
- **Harvest is breaking, never Use.** Use does not harvest anything: its order
  is a Tool, a Usable Voxel, then eating.
- **Breaking a fruit is an ordinary Remove.** A ripe one drops its yield as
  Dropped items, as any removal does, and an unripe one, which declares none,
  drops nothing; either leaves the tree's canopy Material in its place and the
  tree untouched. It is journalled like any removal, with `Cost::Yielded`, and
  undo restores the fruit Voxel and takes the Dropped items back. Every Stage of
  a tree's fruit is one Voxel to the journal (`canonical`), so undo holds after
  the fruit ripened; it is refused once the tree has grown new fruit in that
  cell, by the ordinary mismatch rule. The tree then grows new fruit elsewhere
  on its canopy (re-bearing above) while it stands.
- **Remove of a ripe crop that declares `harvest-to` is a harvest**: it gives
  the yield to the Inventory (what does not fit dropping at the Player), resets
  the Voxel to `harvest-to` and never enters the journal, so a berry bush is
  never destroyed by a left click and fruits again. Remove of an unripe Stage
  breaks it, so a whole berry bush is removed by breaking it at the reset Stage.
  A crop without `harvest-to` (wheat, carrot) is uprooted at any Stage. A
  mushroom or other spreader is picked and removed; it returns only by spreading
  from a neighbour.
- **A crop harvest and a Felling's cascade are Edits that never enter the
  journal**, like a Door toggle. Undo after cutting a trunk restores the Voxel
  the Player cut and no more.

## Rejected

- **Growth state in the Overlay or beside the Chunk.** A Stage is already a
  Material, which saves, meshes and lights with no new layer; per-Voxel state
  would be carried by every Chunk, save and Mod Command path.
- **Plants as Entities.** Entities are few, free-moving and saved as records; a
  field is millions of fixed cells that the Chunk already stores.
- **An Agenda entry per plant.** The Agenda holds one entry per Voxel at a fixed
  capacity and overflows into Rescans. A meadow would flood it and every idle
  Voxel would cost memory. Sampling bounds the cost by Chunks held and costs
  nothing for a Chunk with no Plant Material, at the price that a Stage takes
  its `mean-seconds` only on average and no faster than 16 s.
- **A stored tree identity** (a tree id per Voxel, or a registry of trees),
  which every save, undo and Mod Command path would have to keep in step for
  what the Voxels already show. Grown Materials and connectivity derive it, as
  Connections and Doors are derived
  ([ADR-0010](0010-connections-are-derived-not-stored.md)).
- **Harvest by Use** (a `G Harvest` prompt on ripe fruit), which a Player found
  confusing beside breaking, the one way every other Voxel is taken.
- **A fruit that resets to an earlier Stage** (`harvest-to` on fruit), which
  kept a fruit site fixed for ever instead of the tree bearing where it has
  room.
- **A Mod callback on growth or harvest**, which is the per-use host call
  ADR-0001 forbids.
- **Pausing the clock for evicted Chunks, or catching up on return**, which
  would make a World's state depend on how far the Player walked.

## Consequences

- A pending Felling is not saved. A World closed mid-fall leaves what was queued
  standing until a neighbouring change asks again.
- A Felling region beside a Voxel the Server does not hold waits: it is neither
  broken nor dropped from the next ask, and it breaks once the Chunk is held.
- The Felling size bound is `2 × reach + 3` Voxels wide (plus rounding slack for
  a branched tree) and `trunk-height-max + max(canopy-radius, lobe-radius) + 3`
  Voxels tall. A region past it does not break.
- Crowns and lobes are parametric ellipsoids of 0.8 vertical ratio and limbs are
  stepped lines, so a pine is a cone of tiers and an acacia an umbrella of
  lobes; anything more particular (buttresses, curved limbs, a drooping tip)
  waits for Structures to give trees templates.
- A limb resting on rising ground holds the tree up like any trunk Voxel that
  stands on something neither Air nor grown, and lobes touching another part's
  lobes keep a cut part up, because Felling joins grown Voxels through faces. A
  cut limb drops a sapling like any region, once.
- Branching costs Voxels: at 0.25 m the shared oak is 6590 Voxels against 3341,
  and lands in 10.2 ms of CPU against 6.1 (0.29 against 0.17 ms at 1.0 m and
  1.58 against 0.86 ms at 0.5 m), 65% of a 15.6 ms step, so `TREES_PER_STEP`
  stays 1. A felled step breaks the same 32 Voxels and costs 0.12, 0.14 and 0.74
  ms. Generating a surface Chunk of a 16-feature Biome whose three trees branch
  costs 0.73, 0.41 and 0.54 ms against 0.43, 0.25 and 0.20 with plain trees
  (0.12, 0.08 and 0.08 bare); a tree's cells are deduplicated in a flat box,
  since hashing each lobe cell cost four times as much.
- A flat Shape costs nothing against the Chunk's shaped-Voxel cap, so a dense
  meadow is bounded only by the Chunk's own Voxels. The Sample cost of a dense
  meadow is the number to watch, and a Chunk with no Plant must cost nothing
  measurable.
- A spreader's density cap counts what is held and bounds the neighbourhood of a
  Voxel only when it lands, so beside an unheld Chunk a field may overshoot it.
  It bounds a neighbourhood and never a field's extent: a 16 Voxel patch of the
  shared tuft at `mean-seconds` 120 and `density` 6 grew to 64% of a Chunk in 4
  hours. Ground cover therefore spreads slowly (900 s and up) and patches come
  from the Biome's clump.
- One Shape serves both of a fruit's Stages, so unripe and ripe differ in
  Material colour alone.
- A Biome's frozen Plant form (stages, column, tree, fruit, vines and branching,
  by name) pins what generation reads, not growth: growth reads the Game's Plant
  rows, which the World already freezes with its exact builds.
- Plants are Server state replicated as Chunk edits; a Client draws what it is
  sent and decides no growth ([ADR-0002](0002-client-server-from-day-one.md)).
