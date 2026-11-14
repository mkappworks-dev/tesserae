# Plants are declared kinds the Server grows

Status: accepted.

Step 18 makes vegetation live. A Game or Mod declares **Plants** as data, each
with a closed kind (`crop`, `tree`, `column`, `spreader` or `climber`), and the
Server grows, harvests and fells them in native code. No Mod is called when a
Plant grows or is picked ([ADR-0001](0001-wasm-mod-boundary.md)); a new kind
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

The four numeric constants are starting values until the benchmark pass has
measured them.

**Kinds.** A `crop` is one Voxel through 2 to 8 ordered Stages, the last ripe,
with an optional `harvest-to` Stage a Use on the ripe one resets to. A `tree` is
a sapling that grows a parametric tree as one all-or-nothing group. A `column`
adds one Voxel at the end of its run, up or down, to a height in metres,
optionally standing in a fluid (kelp). A `spreader` copies itself onto a
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

## Trees: grown Materials against placed look-alikes

A tree's trunk and canopy are **grown Materials**, made only by growth and
worldgen. A Game declares a placed Wood and a grown trunk of the same look, and
a Player builds only with the placed one. Felling tells a grown tree from a
build by identity-free Voxels: no Voxel records which tree it belongs to. When a
grown trunk Voxel changes, the Server evaluates the **Felling** region: the
grown Voxels of that Plant's trunk, canopy and fruit Materials, connected
through faces, that no longer reach a trunk Voxel standing on something that is
neither Air nor any Plant's. Only grown Materials take part, so a cabin of
placed logs and leaves, however alike in look, is never in a region and never
falls. A region wider than `2 × canopy-radius + 3` Voxels or taller than the
tallest trunk plus `canopy-radius + 3`, or one beside a Voxel the Server does
not hold, does not break.

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
only if every cell of the group is Air or the tree's own Material, and otherwise
stays a sapling and Samples again.

Fruit replaces canopy Voxels chosen by a hash of seed, position and `density`
(at most 0.3). A fruit Voxel ripens through its grown Stages like a crop, with
the tree's light and Climate, light read as the brightest of the Voxel and its
six neighbours since fruit sits in canopy. Use picks a ripe one and resets it to
`harvest-to`; the tree is untouched.

**Vines are generated only in worldgen.** A tree row may name a `climber` and a
density and length. Worldgen hangs it from the outer canopy by a pure function
of the tree, seed and Voxel size, whole across Chunk borders, and a growing
sapling never does. Once placed, vines live by the climber's rules.

## Harvest, undo and what is not journalled

Remove of a plant Voxel drops that Stage's declared yield, with each count a
hash of seed, position and Stage inside its range, never a draw from a
generator. A Stage that declares none drops the Item that places it. Creative
drops the same. A removal's drops ride the undo entry as `Cost::Yielded`
([ADR-0023](0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md)):
undo takes back what still lies there and spends the rest from the Inventory,
refusing whole without it.

- **A crop's Stages are one Voxel to undo and redo**, as a Door's two Materials
  are, so undoing a Remove or a Seed placement works after the crop grew.
- **A tree is the exception.** A trunk is not in the sapling's chain, so undoing
  the Seed's placement after the tree grew is refused by the journal's ordinary
  mismatch rule and never half-removes a tree.
- **Use harvests, picking fruit and a Felling's cascade are Edits that never
  enter the journal**, like a Door toggle. Undo after cutting a trunk restores
  the Voxel the Player cut and no more.

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
- **A Mod callback on growth or harvest**, which is the per-use host call
  ADR-0001 forbids.
- **Pausing the clock for evicted Chunks, or catching up on return**, which
  would make a World's state depend on how far the Player walked.

## Consequences

- A pending Felling is not saved. A World closed mid-fall leaves what was queued
  standing until a neighbouring change asks again.
- Canopies are parametric ellipsoids of 0.8 vertical ratio, so a pine cannot be
  a cone and an acacia cannot be an umbrella until Structures give trees
  templates.
- A flat Shape costs nothing against the Chunk's shaped-Voxel cap, so a dense
  meadow is bounded only by the Chunk's own Voxels. The Sample cost of a dense
  meadow is the number to watch, and a Chunk with no Plant must cost nothing
  measurable.
- A spreader's density cap counts what is held and bounds the neighbourhood of a
  Voxel only when it lands, so beside an unheld Chunk a field may overshoot it.
- One Shape serves both of a fruit's Stages, so unripe and ripe differ in
  Material colour alone.
- A Biome's frozen Plant form (stages, column, tree, fruit and vines, by name)
  pins what generation reads, not growth: growth reads the Game's Plant rows,
  which the World already freezes with its exact builds.
- Plants are Server state replicated as Chunk edits; a Client draws what it is
  sent and decides no growth ([ADR-0002](0002-client-server-from-day-one.md)).
