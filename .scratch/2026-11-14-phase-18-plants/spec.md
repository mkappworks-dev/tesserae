Status: ready-for-agent

# Phase 18: Plants

DESIGN.md roadmap step 18: growth on the Server tick, placement rules and
harvesting. Depends on step 13 (Biomes, done) and builds on the Inventory,
dropped items and Use work of steps 15 and 16. The Engine adds the mechanisms; a
Game or Mod supplies Plant data.

Vocabulary: **Plant**, **Stage**, **Growth**, **Sample**, **Seed** and
**Felling** are new and are added to [GLOSSARY.md](../../GLOSSARY.md) by ticket
15, with the decision record ADR-0027 and an amendment to
[ADR-0006](../../docs/adr/0006-shapes-are-table-data.md). Related decisions:
[ADR-0013](../../docs/adr/0013-fluid-and-fire-live-beside-the-voxel.md),
[ADR-0014](../../docs/adr/0014-simulation-advances-in-bounded-logical-steps.md),
[ADR-0022](../../docs/adr/0022-biomes-are-column-content-and-a-world-pins-its-generator.md),
[ADR-0023](../../docs/adr/0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md)
and
[ADR-0026](../../docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md).

## Problem Statement

A World's vegetation is inert. Biome surface features scatter a Material once at
generation and it never changes: grass never spreads, a melon never ripens,
nothing can be planted and a tree is a hand-built stack of wood and leaves. A
Survival player has nothing to farm, so food comes only from what generation
happened to place, and Terra's loop has no renewable resource. Plants that do
exist draw as full cubes with a cut-out texture, so a tuft of grass or a stalk
of wheat reads as a block, and an Item for one in the hand, the hotbar or on the
ground is a cube too.

A Game author cannot declare a crop, a sapling or a vine. The contract has no
rows for them, and per [ADR-0001](../../docs/adr/0001-wasm-mod-boundary.md) the
answer cannot be a Mod callback on every growth.

## Solution

A Game or Mod declares **Plants** as data. A Plant is a closed kind the Server
runs natively:

- `crop`: one Voxel through an ordered chain of **Stage** Materials, optionally
  resetting to an earlier Stage when harvested.
- `tree`: a sapling that grows a parametric tree, trunk and canopy sized in
  metres, as one all-or-nothing group. A tree may bear fruit: some canopy Voxels
  ripen through grown fruit Stages, are broken to harvest without cutting the
  tree, and the standing tree grows new fruit elsewhere on its canopy.
- `column`: grows up or down one Voxel at a time to a height in metres (sugar
  cane, cactus, bamboo, kelp).
- `spreader`: copies itself onto a neighbouring Voxel that meets its rules
  (grass, moss, mushrooms, flowers).
- `climber`: grows along the faces of solids, drawn as a flat quad against the
  face (vines).

A Plant's state is its Material. A Stage is a Material, so saving, meshing,
light and collision need nothing new, and growth is the Server changing one
Voxel's Material through the one World change. Nothing is stored per plant.

Growth is a Chunk-sampled tick on the Server's Logical step, deterministic from
the World seed. Planting needs a **Seed** Item and soil; growth needs light,
Climate and, where declared, water. Harvest is breaking (Remove) at any Stage, which drops
that Stage's yield; Use harvests nothing. Breaking a ripe crop with `harvest-to`
yields and resets it. A
plant loses its support, a trunk is cut, and what depended on it breaks and
drops.

Plants draw as crossed quads, or one flat quad for a climber, through a new
`cross` Shape form, and every Item of one is a flat sprite in the hand, the
hotbar, the Inventory, the placement ghost and on the ground.

Biomes may scatter Plants, trees included, at generation. The `flora` Mod gains
rows for grass, kelp and the oak; Terra gains wheat, a berry bush, sugar cane,
mushrooms and vines.

## User Stories

Planting and placement

1. As a Survival player, I want to plant a Seed on soil, so that I can start a
   crop of my own.
2. As a player, I want a refusal with a reason when I plant on the wrong ground,
   so that I know what the Plant needs.
3. As a player, I want to plant in the dark and have the Plant wait, so that
   planting never fails for a reason that changes by the hour.
4. As a player, I want a ghost of the Plant at the target before I place, so
   that I see what I am about to plant.
5. As a player, I want to place a Layout of Seeds as one action, paid up front
   and refused whole if any cell fails, so that planting a row works like any
   other Layout.
6. As a Creative player, I want every Plant's Seed in the Catalogue and no
   unplaceable later Stage, so that I cannot place a ripe crop that never grew.

Growth

7. As a player, I want a crop to ripen through visible Stages over minutes, so
   that I can see it progress.
8. As a player, I want growth to need light, so that a crop in a cave or under a
   roof waits.
9. As a player, I want growth to need the right Climate, so that wheat does not
   thrive in a desert.
10. As a player, I want a Plant that needs water to grow only near it, so that
    irrigating matters.
11. As a player, I want a Plant that fails a condition to hold its Stage and
    never die from it, so that I can leave a farm and come back.
12. As a player, I want a Plant to grow only while its Chunk is loaded, so that
    time away does not skip it.
13. As a player, I want sugar cane to grow upward to a height in metres, so that
    it is as tall at every Voxel size.
14. As a player, I want kelp to grow in water, so that the sea is not static.
15. As a player, I want grass to spread onto nearby bare soil, so that a
    trampled field greens over.
16. As a player, I want mushrooms to spread in the dark, so that caves fill
    with them.
17. As a player, I want a sapling to grow into a tree, so that I can plant a
    forest.
18. As a player, I want a sapling with no room to stay a sapling and try again,
    so that it never grows into a wall.
19. As a player, I want vines to grow along a wall and hang from an overhang, so
    that ruins gather them.

Harvest and drops

20. As a player, I want breaking a ripe crop to give its produce and a Seed, so
    that I can replant.
21. As a player, I want breaking a young crop to give back at most its Seed, so
    that I do not farm unripe plants.
21b. As a player, I want some trees to bear fruit I break to collect and eat,
    leaving the tree standing and growing new fruit elsewhere on its canopy, so
    that an orchard is a renewable food source.
22. As a player, I want breaking a ripe berry bush to give berries and leave the
    bush at an earlier Stage, so that it keeps fruiting.
23. As a player, I want one way to harvest, breaking, with no separate Use
    prompt, so that nothing is confusing.
24. As a player, I want the same yield in Creative as in Survival, so that
    Creative can test a farm.
25. As a player, I want undo to give back a Seed I planted and re-place a plant
    I broke, even after it grew a Stage, so that undo stays reliable.
26. As a player, I want a plant whose soil I dig away to break and drop its
    Seed, so that nothing floats.

Trees

27. As a player, I want a tree to be the same Voxels at any Voxel size, so that
    its height and canopy are in metres.
28. As a player, I want cutting the trunk of a grown tree to drop the part above
    the cut, canopy and all, so that I can fell it.
29. As a player, I want a felled tree to break over a few moments and drop its
    Items, so that a big tree does not stall the game.
30. As a player, I want a cabin I built from placed logs, leaves or both to
    stay standing when I cut one log, so that building is never felled by
    accident.
31. As a player, I want undo after cutting a trunk to restore only the Voxel I
    cut, so that its limit is clear.

World and look

32. As a player, I want meadows, forests and kelp beds in a new World, so that
    it already looks alive.
33. As a player, I want a tree that crosses a Chunk border to be whole, so that
    no tree is sliced.
34. As a player, I want a World saved before Plants to keep its bare ground, so
    that an update does not reshape it.
35. As a player, I want a Plant drawn as crossed quads, so that it reads as a
    plant and not a block.
36. As a player, I want to walk through a Plant, so that grass does not block
    me.
37. As a player, I want to aim at and break a Plant I walk through, so that I
    can still harvest it.
38. As a player, I want a Plant's Item drawn flat when I hold it, preview it,
    look at it in the Inventory and see it on the ground, so that a Seed or a
    sapling is not a cube.
39. As a player, I want produce such as wheat and berries drawn flat from its
    own texture, so that it is not a coloured cube.
40. As a player, I want light to pass through a Plant, so that a field does not
    cast shadow it should not.

Authors

41. As a Game or Mod author, I want to declare a Plant as data with no code, so
    that a data-only Mod can supply one.
42. As a Game or Mod author, I want a load error naming the Plant when a Stage,
    Item, soil or Material does not resolve, so that mistakes are caught at
    load.
43. As a Game or Mod author, I want rates in seconds and sizes in metres, so
    that a Plant behaves the same at every Voxel size.
44. As a Game or Mod author, I want a Biome's surface features to name a Plant,
    so that I choose what grows where.
45. As a Game or Mod author, I want the Engine to ship no Plant, so that a World
    with no Game has none.
46. As an author of the `flora` Mod, I want grass, kelp and the oak declared
    once, so that every Game that uses `flora` grows them.

## Implementation Decisions

**Contract: line 0.3 is amended in place** (it is unreleased; see the "Amended
before release" section of the contract-compatibility doc). The Declaration
gains one list, `item-def` an optional field and `shape-def` a new form.

- `plants`, from any Mod, at most 128 per Mod and 512 per World. A row has a
  name (local to the Mod; the engine adds `<mod id>:`), a closed `kind`, and the
  common fields below. Names resolve after every Mod has declared. An unknown
  Material, Item or tag is a load error naming the Plant.
- Common fields: `seed`, the Item that places the first Stage (its `material` is
  that Stage's Material); `soil`, a list of Materials or Material tags the
  Plant stands on or beside; `light`, the least light level at its Voxel to grow
  (default 0); `climate`, an optional temperature and humidity range from 0 to
  1; `water`, an optional reach in metres within which a source or flowing fluid
  must lie; and `yields`, per Stage, a list of Items with a count range (empty
  for none).
- `crop`: `stages`, 2 to 8 Materials in order, each with `mean-seconds` to reach
  the next; the last Stage is ripe. `harvest-to`: an optional earlier Stage a
  Remove of the ripe Stage resets to.
- `tree`: `trunk` and `canopy`, the tree's **grown Materials**,
  `trunk-height-m` as a range up to 32 m, `canopy-radius-m` up to 8 m, and
  `mean-seconds` for the sapling to grow. The sapling Material is the seed's. A
  grown Material is made only by growth and worldgen: it gets no implicit Item,
  cannot be placed or taken from the Catalogue, and is named by no Item and no
  other Plant, as a Door's open Material is. Its removal drops `trunk-yield` and
  `canopy-yield`, lists of Items with a count range (the Game's placeable log
  and leaves), and `sapling-yield`, a count range of at least 1 of the tree's
  Seed, given once for every tree cut down. A Game therefore declares a placed Wood
  and a grown trunk of the same look, and a player builds only with the placed
  one.
  A tree may declare `fruit`: `stages`, 2 to 4 grown Materials in order, each
  with `mean-seconds` to reach the next; `density`, the share of canopy Voxels
  from 0 to 0.3 that carry fruit, chosen by hash of seed and position when the
  tree grows or is generated; `regrow-seconds`, at least 16, the mean time
  between new fruit on a standing tree; and `yields` per Stage (the ripe one gives the Game's food Item). Fruit Stages
  are grown Materials like the canopy, drawn as a small 3D apple Shape of boxes
  (red when ripe, green when unripe) and never as a flat quad or a cube: they replace canopy Voxels, are part of
  the tree, have no implicit Item and cannot be placed.
- `column`: one Material, `max-height-m` up to 32 m, `direction` up or down,
  `mean-seconds` per added Voxel. Water-standing kinds name the fluid in
  `water`.
- `spreader`: one Material, `reach-m` from 1 to 8, `mean-seconds` to attempt one
  spread, and a `density` cap, the most such Voxels within `reach-m`, so a field
  does not fill solid.
- `climber`: one Material, `mean-seconds` per added Voxel, and a `length-m` cap.
- `item-def` gains an optional `texture`, a path in the Mod's art, so an Item
  with no placeable Material (wheat, berries, cane) draws as a flat sprite. None
  keeps the cube of `color`.
- `shape-def` gains a `form`: `box` (today's nodeboxes), `cross` (two diagonal
  quads through the Voxel's centre) or `plane` (one quad against a face, for
  climbers). A flat form still declares boxes, which decide selection and where fluid
  can stand, so kelp is waterlogged where it grows. Both forms are walked
  through unless the Shape declares collision, may seal no face, cannot
  connect, and cost nothing against the Chunk's shaped-Voxel cap; light
  passes by the Material's own dimming, so a plant's Material is translucent. Data-only Mods get
  the same rows as `[[plants]]` in their content file.
- A Material is at most one Stage of one Plant; a later Stage gets no implicit
  Item and cannot be placed or taken from the Catalogue, as a Door's open
  Material cannot.
- `surface-feature` gains `plant`, naming a Plant in place of a Material, with
  the same spacing. Exactly one of the two is set.

**Pure rules, in `tesserae-core`** (no Bevy, no wgpu)

- The Plant table, resolved and validated, lives beside the Item and voxel-use
  tables, with the Stage-to-Plant lookup the journal and the Server use.
- The Sample kernel: given the World seed, a Chunk's coordinate, the Logical step
  and a sample count, it returns the Voxel positions drawn and, for a Voxel,
  whether its roll passes, from a hash of the same inputs. It never reads the
  World. Equal inputs give equal outputs at every worker count.
- The growth rules: for a Voxel and a read-only view of its neighbours, light,
  Climate and fluids, whether it may grow and what change it proposes. Tree
  shapes, column runs, spread targets and vine attachments are pure functions of
  a Plant row, a position in metres and the Voxel size. Nothing re-implements
  Shape passage or Climate.
- The support rules: whether a plant Voxel's soil, attachment or fluid still
  holds, and the Felling region: the Voxels of one tree's trunk and canopy
  Materials connected through faces that no longer reach a grounded trunk Voxel,
  bounded by the Plant's declared size in metres.

**Server**

- Growth is a Sample tick. Each Logical step every held level-0 Chunk whose
  palette holds a Plant Material draws `PLANT_SAMPLES` positions from the
  kernel. A drawn plant Voxel rolls against its Stage's `mean-seconds` as
  `min(1, interval ÷ mean)`, where `interval` is the mean time between Samples
  of one Voxel, `CHUNK_VOLUME ÷ (PLANT_SAMPLES × 64)` seconds (16 s at 32). A
  `mean-seconds` below that interval is a load error, so the interval is the
  documented ceiling on growth speed. The starting numbers are tuned by ticket
  04's bench.
- A passing roll checks the Plant's conditions against the Server's Snapshot:
  soil, light at the Voxel, Climate at the column in metres, water. A failing
  condition leaves the Voxel as it is. A Chunk whose light solve is pending or
  whose neighbour is unheld is skipped, never read as Air.
- Growth lands through `set_occupants`, one change a Voxel, under the
  per-step bound `PLANTS_LANDED_PER_STEP` and a separate `TREES_PER_STEP`. A
  change past a bound is deferred to the next step with its roll kept. A change
  is dropped and its Voxel left to Sample again if any event of its step
  touched what it read, if a Chunk's Lifetime changed, or if its Voxel no longer
  reads what it was computed from.
- A Sample tick lands after every event of its step's index, as ADR-0014
  specifies, so equal state and events at equal steps give equal state.
- Only held Chunks Sample. A Chunk held and then evicted resumes as it was,
  never credited the time it was gone ([ADR-0013](../../docs/adr/0013-fluid-and-fire-live-beside-the-voxel.md),
  "offline wall time is never added").
- Placing a Seed is Place from the selected slot, as any Item: the Server checks
  that the Item places the Stage Material and spends one. A Plant's `soil` rule
  is Edit validation, in both modes and for a Layout, with a refusal reason
  naming what the Plant needs ("Wheat needs farmland."). Light, Climate and
  water are never placement rules.
- Support: when a plant Voxel's soil, attachment or fluid changes, the Server
  wakes it through the existing wake-on-change and re-reads its support. A
  failing Voxel breaks, dropping the Seed, in the same Logical step. Nothing is
  polled.
- Harvest: Remove of a plant Voxel drops that Stage's declared `yields`, with
  each count a hash of seed, position and Stage inside its range, never a draw
  from a generator. The Stage's yield and its Dropped item ids ride in the
  undo entry like any removal's ([ADR-0023](../../docs/adr/0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md)).
  Creative drops the same.
- Remove of a ripe crop with `harvest-to` gives its yield and sets the Voxel to
  the `harvest-to` Stage; it is an Edit that never enters the undo journal, like
  a Door toggle. Use harvests nothing: its order is a Tool, a Usable Voxel, then
  eating, and the aim answer's `use_aim` names no Harvest verb.
- A fruit Voxel ripens through its Stages by the Sample tick like a crop, with
  the tree's light and Climate conditions. Remove of a fruit Voxel is an
  ordinary journalled removal: it drops its Stage's yield (none unripe) and
  leaves the tree's canopy Material in its place, and a Felling drops the yield
  of every fruit Voxel it breaks. A standing tree re-bears: each of its canopy
  Voxels Samples at a mean of `regrow-seconds` times the tree's expected canopy
  cell count and, passing, becomes unripe fruit when its light and Climate hold
  and the tree's fruit is below `density` times that count. A felled tree grows
  none.
- Undo and redo treat every Stage of one crop as the same Voxel, as a Door's two
  Materials are. A `tree` is the exception: a trunk is not in the sapling's
  chain, so undoing the seed's placement after the tree grew is refused by the
  journal's ordinary mismatch rule, never half-removing a tree.
- `tree` growth is one group: the sapling's cell, the trunk run and the canopy
  cells, all of them Air or already the tree's own Materials, or the group does
  not land and the sapling Samples again. It lands as one change under
  `TREES_PER_STEP` and relights once.
- `column`: the Voxel above (or below, per `direction`) the end of the run grows
  one more while the run is under `max-height-m` and the target is Air or the
  fluid the Plant stands in. `spreader`: a Sample that passes picks a target
  within `reach-m` by hash, and spreads if the target meets the Plant's soil and
  light rules and the density cap holds. `climber`: spreads to an adjacent face
  of a solid, or hangs below an overhang, within `length-m`.
- Felling: when a grown trunk Voxel changes, the Server wakes its neighbours and
  evaluates the Felling region: the grown Voxels of that tree, connected through
  faces, that no longer reach a grounded grown trunk (one standing on a Voxel
  that is not grown). Only grown Materials take part, so what a player placed,
  however alike in look, is never in a region and is never felled: a cabin of
  placed logs, with or without leaves, stays. The region is bounded by the
  Plant's declared size in metres; one that exceeds it does not break. It
  breaks top-down over successive Logical steps under
  `FELLED_PER_STEP`, each Voxel dropping its Item like any removal. A Felling also
  drops `sapling-yield` saplings, at least one, once for the tree, where the
  trunk was cut and by a hash of seed and position, so every tree cut down
  can be replanted. Breaking a lone leaf drops only `canopy-yield`. The cascade is not an Edit the journal
  records: undo restores the Voxel the Player cut and no more, like a Door
  toggle. Falling as a body, in a direction, is step 24's.
- Mod Commands, burning and Felling reach plant Voxels through the same
  `set_occupants`; there is one write path.

**Worldgen**

- A Biome's `surface-features` may name a Plant. Generation places it at a
  hash-drawn Stage (a tree fully grown) under the existing rules: the column's
  top a full cube of the Biome's surface Material, air above, inside the global
  surface bound, never under a fluid, on a stair or slab or in a cave mouth. A
  kelp `column` stands in the lake.
- A tree is a pure function of the seed and a column position in metres, so a
  Chunk generates the trunk and canopy Voxels of every neighbouring column that
  fall inside it, scanning the Chunks within `canopy-radius-m`. The bound on the
  radius (8 m) bounds the scan.
- Level 0 only, as ADR-0022's surface features are. No new Generator version: a
  Biome with no Plant feature runs no new pass, so version 2 Worlds generate as
  before and tests pin digests recorded before Plants existed. The Plant rows
  ride in the World's Biome snapshot, so a Game that gains a Plant later cannot
  change a World saved earlier.

**Save**

- A Plant Voxel is a Material; growth state is the Stage. A tree is Voxels.
  Nothing is added to the format, the Overlay or the Agenda, and no generator
  version moves. Stage Materials save by name through the Registry like any
  other.
- A World saved with a Plant's Material whose Plant has since been removed from
  its Game still loads: the Voxel is a Material like any, and stops growing.
- The Plant rows are Declaration content; the World already freezes its Mods and
  their exact builds.

**Client** (after the mockup's step 18 section is approved: ticket 01)

- Meshing: `cross` and `plane` forms are drawn from the Material's texture, in
  the cutout pass, with the mesher emitting no face-merging across them and
  neighbouring faces not hidden by them. Climbers take the face their
  Orientation names.
- Every Item of a `cross` or `plane` Shape, and any Item with a `texture`, is
  drawn as a flat sprite of that texture: icons in the Inventory, hotbar,
  Catalogue and Crafting, the hand, and a Dropped item, upright and turning at
  the Item's declared size. Others are unchanged.
- The placement ghost is the Shape at the target; a Layout of Seeds shows each
  cell.
- There is no Harvest prompt: harvest is breaking. No new pane.
- The Client reads no World storage and decides no growth: it draws what the
  Server sent, and Stage changes arrive as ordinary Chunk edits.

**Everything physical is in metres**: spread reach, tree and column sizes,
water reach, canopy radius and every rate in seconds, converted to Voxels and
Logical steps when the World opens ([ADR-0003](../../docs/adr/0003-voxel-size-is-per-world.md)).

**Content**

- `flora` declares grass as a `spreader`, kelp as a `column`, an oak `tree` over
  its wood and leaves, and flowers as `spreader`s. It declares their Shapes and
  textures; `meadow`, `ember` and `tidewater` gain them, and need new Seeds
  only if they want to plant them.
- Terra also declares an apple tree: the oak's shape with apples ripening in its
  canopy that is broken to collect them, and apples that restore hunger.
- Terra declares wheat (`crop`, with Seeds and produce), a berry bush (`crop`
  with `harvest-to`), sugar cane (`column`), mushrooms (`spreader`) and vines
  (`climber`); the Biome features place them; the Recipes make food from them.
  Terra's standalone Wood surface feature is removed: Wood comes from breaking
  a grown oak, which drops the placed Wood Item, and Terra's temperate Biome
  places oaks in its place so the Workbench, Chest and Door chain stays
  reachable in a short walk.
- Stage art is drawn by `scripts/generate-mod-art`, never by hand.

## Testing Decisions

A good test drives external behaviour: intents in, Server messages, save
contents or what is on screen out. It never asserts on private helpers, ECS
layout or how samples are drawn.

Four seams, all existing:

1. **The pure kernel and rules in `tesserae-core`.** Prior art: the flow kernel
   and Inventory rules. Covers: the Sample draw is deterministic and
   independent of worker count; the roll probability and the `mean-seconds`
   floor; conditions (soil, light, Climate, water) including Voxel-size
   independence at 1.0, 0.5 and 0.25 m; tree, column, spread and vine shapes as
   functions of metres; the support and Felling regions, including a cabin of
   placed Materials that look like the tree's, a placed leaf touching a grown
   trunk, and a region over the size bound.
2. **The Server headless App.** Prior art: the Inventory, drops, Use and fluid
   tests. Covers: a crop advances only when its conditions hold and holds when
   they fail; Seeds place, spend and refuse by soil, in both modes, for a
   Layout; Remove yields per Stage, Creative included; Remove harvests a ripe
   `harvest-to` crop and breaks fruit to canopy; a standing fruit tree re-bears
   to its cap; undo and redo across a Stage change, across a harvest and across a tree
   growth (refused); growth deferred past the landing bounds and applied the
   next step; a tree that does not fit stays a sapling; support loss breaks and
   drops; Felling breaks top-down under its bound and never touches a cabin;
   equal state and events give equal state across worker counts.
3. **The save round trip and worldgen.** Prior art: the persistence rig and the
   Biome feature digests. Covers: a grown crop and a tree survive reopen; a
   World with a removed Plant opens; generation of Plants and trees, including
   a tree crossing a Chunk border, is whole and Voxel-size independent in
   metres; a version 2 World with no Plant feature generates the recorded
   digests; the coarse levels leave Plants out.
4. **The Client App driven by key presses.** Prior art: the Inventory window,
   hotbar and Door preview tests. Covers: a Seed's icon, hand, ghost and
   dropped sprite are flat; produce with a `texture` is flat and without one is
   the cube; Use offers no Harvest prompt.

Declaration parsing and validation (limits, unknown names, a Material in two
Plants, a `mean-seconds` under the Sample interval, a Stage chain of one) gets
unit tests beside its code.

A bench, run under the rules in the benchmark memory (interleaved CPU-time A/B
against `main`, never wall-clock at load), measures a Sample tick over a
Chunk-dense meadow, a sampled tree growth and a Felling, and fixes
`PLANT_SAMPLES`, `PLANTS_LANDED_PER_STEP`, `TREES_PER_STEP` and
`FELLED_PER_STEP`. A Chunk with no Plant must cost nothing measurable.

Review-only screenshots of the look (offscreen, one `/tmp` folder, never
committed, deleted after review) check the flat sprites against the mockup.

## Out of Scope

- Falling trees as moving bodies, in a direction (step 24), and Voxel gravity.
- Tree templates and structures (step 20): the `tree` is parametric until
  then.
- Leaf decay, rot, withering, and plants dying from failing conditions.
- Weather, rain and seasons as growth inputs (step 17); bonemeal and
  fertiliser; farmland tilling beyond what a `soil` Material row says.
- Breeding, mob grazing and pollination.
- Fire spread rules: plants burn only through a Game's combustion rows.
- A third Climate axis.
- Coarse (level 1 and above) Plants.
- Mod-drawn UI or a Mod callback on growth or harvest.
- Network transport: Plants are Server state, replicated as Chunk edits.

## Further Notes

- Ticket order: the mockup, the Plant table and the `cross` Shape start at
  once; the Sample kernel follows the table; the other kinds follow the kernel;
  Seeds, harvest and support follow the table and kernel; worldgen follows the
  kinds; Client drawing follows the mockup and the Shape; content follows
  everything; docs come last.
- Risks: the `cross` Shape form crosses the mesher, collision, light and
  selection at once; Felling tells a grown tree from a build by its grown
  Materials, so a Game declares a placed log and a grown trunk of the same look
  and a player cannot place the grown one; cross-border trees in worldgen are
  the classic place for a seam; a dense meadow's Sample cost is the number to
  watch.
- The mockup is not copied into the engine.

## Amendment 2026-11-14: more vegetation, vines in worldgen

The maintainer asked for many tree species, fruit and vegetable plants and
vines across every Biome of every in-repo Game. Ticket 16 adds them: birch, pine,
palm and a second fruit tree beside the oak and apple, wild vegetables, and
vegetation for `meadow`, `ember`, `tidewater` and `lumen`. Vines are generated
by worldgen only, hung from the canopy of trees that declare `vines` in their
row, so "climbers in worldgen" is no longer a gap; a growing sapling never makes
them. Everything else is content on the existing contract.

## Amendment 2026-11-15: bounded spreading, vine crops and fields

The maintainer approved the mockup's "Spreaders and fields" section
(`/vegetation`). Three changes follow; tickets 20 and 21 are built now, ticket
22 is specified and left for later.

- **`min-neighbours` on a spreader (ticket 20).** Ground cover still creeps: a
  `density` cap bounds the crowd around a new Voxel and never how far a field
  advances. An optional spreader row field, `min-neighbours` (default 0, no
  rule), makes a Sample seed a target only where at least that many Voxels of
  the same Plant stand within `reach-m` of it, counting the seeding Voxel. A
  patch then fills its holes and never advances. A lone placed seed would
  never grow under a strict count, so the requirement is capped by how crowded
  the seeding Voxel is itself: the target must be at least as crowded as the
  source, up to `min-neighbours`. A lone seed grows into a compact clump and
  stops; a generated patch, whose Voxels all see the full count, only fills.
- **Vine crops (ticket 21).** A `crop` may declare `fruit-vine`: its ripe Stage
  bears fruit on the ground around it, as a tree bears fruit on its canopy.
  Fruit stands within `reach-m` of the vine (1.5 m), never touching another
  (`gap-m`, 0.5 m, at least a Voxel of ground), up to `max-fruit` (5) per
  vine. Fruit never seeds, so a cluster does not creep; breaking one lets the
  vine bear another. Watermelon and pumpkin join the `crops` Mod and the Biome
  features, and worldgen stands a vine with its first fruit, whole across Chunk
  borders.
- **Fields (ticket 22, not built).** Wild fields group ground cover by region
  so a patch is mostly one species, and planted fields are rows of one crop
  with paths and a water ditch beside water in meadow Biomes.

The contract changes stay in the unreleased 0.3 WIT, with optional fields only:
a Mod that sets neither is unchanged. The World's Biome snapshot gains one
optional kind for a generated vine; a World that has none reads and generates as
before, so `FORMAT_VERSION` and `GENERATOR_VERSION` stay.
