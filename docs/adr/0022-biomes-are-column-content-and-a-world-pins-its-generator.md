# Biomes are column content chosen by Climate, and a World pins its generator version

Status: accepted.

Terrain had one ground, one cave field and one haze colour. Step 13 lets a Game
or Mod declare Biomes, chosen from Climate, without letting a later change
reshape a World that already exists.

A Biome owns the ground Materials, ores, cave tuning and the sky and fog colours
of the columns it covers. It does not own surface height, relief or sea level.
Coarse levels are generated from one height per column and a shifted sea level
([ADR-0011](0011-coarse-levels-are-derived.md)), and the Server's vertical
streaming band and open-sky relight depend on one global bound on the surface. A
Biome that moved the surface would break all three.

Climate is two noise fields, temperature and humidity, sized in metres, and a
Biome is a point in their square. The nearest point wins, with the border warped
by noise. Everything is a pure function of the seed and a position in metres, so
it holds at every Voxel size ([ADR-0003](0003-voxel-size-is-per-world.md)) and
for every level's generator. Ground and ores switch hard at a border. Cave
tuning and the colour pair blend smoothly, because a step in either is visible.

A World is pinned twice. Its saved parameters carry a snapshot of its Biome
table, so a Biome a Game gains later cannot change the World. Its Generator
version, kept in the save and passed to the generator, moves from 1 to 2: a
version 1 World generates exactly as before, with no Biomes. The save accepts
both versions and refuses any other. We save parameters and a number, never an
engine-internal layout, because a pinned layout is what broke Minecraft's 1.17
to 1.18 upgrade.

The Server sends the Client the sky and fog pair for where the Eye is, and the
Client eases to it. The Client holds no generator, so the Server stays the only
source of World facts.

Rejected: Biomes that shape height, which Minecraft does and which this design
cannot afford; a six-axis Climate, which is harder to author and can be added
later as a third axis without changing the declaration; and keeping a second
generator forever, as Hytale does, because snapshotting the table plus one
version number is enough while the algorithm for version 1 stays in the code.

The fixed noise octaves and the border warp are part of version 2's layout.
Changing them later needs version 3.

## Surface features

Amended 2026-11-05, from the phase 15 spec's ticket 21. A Biome also owns its
surface features: at most four solid Materials, each with a mean spacing from 4
to 1024 metres, that generation stands one Voxel above the Biome's surface
Material. A column's draw is a hash of the folded seed and its position in
metres, and each feature's chance is the square of Voxel size over spacing, so
the density is the same at any Voxel size. A feature stands only where the
column's top is a full cube of the Biome's surface Material and the cell above
is still air after lakes and smoothing, with its floor inside the global surface
bound: never under a fluid, on a stair or slab, or in a cave mouth. So it never
moves the surface the streaming band and open-sky relight depend on.

- **Pinned with the snapshot.** Features ride in the World's Biome table like
  the ground and the colours. A World saved before a Game declared one keeps its
  bare ground; only new Worlds get them.
- **No new Generator version.** A Biome without features runs no pass, so a
  version 2 World that has none generates exactly as before; tests pin digests
  recorded before features existed.
- **Level 0 only.** The coarse-level sources generate without them, because two
  features in one octant could win a coarse Voxel's vote
  ([ADR-0011](0011-coarse-levels-are-derived.md)). A Chunk a player changed
  keeps the features it held and downsamples like any other Edit.
- **Mods still place no terrain.** The Engine places what the declaration says;
  removing a feature drops its Item like any Voxel.

## Surface feature limit raised to 16

Amended 2026-11-14, from the phase 18 spec's ticket 16. A Biome may declare at
most 16 surface features, up from four; a 17th is a load error that names the
Biome. Four was a placeholder from when a feature was one Material. With Plants
a Biome wants a dozen species, and the other voxel games set no small cap:
Minecraft gives a Biome whole lists of features and Luanti lets a Biome name any
number of decorations.

- **Still inline.** `SurfaceFeatures` stays a fixed `Copy` array. At 16 a
  `Biome` measures 1156 bytes (260 at four) and `WorldgenParams` 19 KiB (4.8 KiB
  at four), vines included, copied only when a table is built or `params()` is
  called, never per column: generation reads features through `&Biome`. A shared
  table behind the Biomes would only be needed past roughly 64.
- **Chances still add up.** A feature's chance is still the square of Voxel size
  over spacing, laid end to end on one draw. If a Biome's chances sum past 1 (16
  features at 4 m spacing and 2 m Voxels do), the earlier declarations take the
  column and the later ones never stand there, identically from every Chunk, so
  the density of a late feature is capped by what the earlier ones leave. A
  Biome that wants all of them keeps its spacings wide enough to sum below 1.
- **Digests unchanged.** The snapshot field is the same list, and a World with
  no Plant feature runs the same passes, so recorded digests hold; neither the
  save format nor the Generator version changes.

Vines are the one addition to a tree's frozen form: worldgen hangs a climber
from the outer canopy by a pure function of the tree, seed and Voxel size, whole
across Chunk borders, and only worldgen does (a growing sapling never does).

## Surface features may clump

Amended 2026-11-14, from the phase 18 spec's ticket 19. A surface feature may
declare a `clump`, a scale in metres (4 to 1024) and a strength (0 to 1), so
trees gather in groves with clearings between and ground cover grows in patches.
Without one a feature scatters evenly, as before. The spacing range widens from
4 m to 1 m, since a jungle's ferns stand a metre or two apart.

- **A density field of position.** With a clump the column's chance is
  multiplied by `1 - strength + strength * w / m`, where `w` is a smooth ramp of
  value noise over a lattice `scale-m` metres apart (seeded by the World seed
  and the feature's place in its Biome) past a cut, and `m` the ramp's mean over
  the plane. The cut leaves `0.85 * strength` of the ground bare, read off the
  noise's own distribution, so a strength of 0.95 packs the feature into patches
  over a fifth of the ground at up to 5 times its chance and 0.6 gives groves at
  up to 1.6 times with thin clearings. The factor's mean over the plane is 1:
  the declared spacing is still the mean spacing, and a test holds it to 5%.
  Amended 2026-11-14 (ticket 19 follow-up, a maintainer report of too much grass
  and flowers everywhere): the first field, `1 + strength * (2t - 1)`, never
  concentrated a feature past twice its mean, so no patch could be denser than 2
  times the cover and ground cover read as an even speckle.
- **Same everywhere.** The field reads only the seed and a position in metres,
  so it is the same at every Voxel size and from every Chunk, and a tree's
  candidate scan evaluates it for the candidate column, so a tree across a Chunk
  border stays whole.
- **Chances still add up.** The early-out bound `most` sums each feature at its
  densest, and the earlier declaration still takes a column when the chances
  pass 1; a clump that raises a ground cover's chance never costs a tree its
  column, because trees are declared first.
- **Digests unchanged.** A feature with no clump runs the same arithmetic and a
  clump of strength 0 multiplies by exactly 1. The snapshot carries the clump as
  an optional field, absent in an older save, so neither the save format nor the
  Generator version changes.
- **Cost.** The noise runs once per clumped feature for a column whose draw
  passed `most`, with no allocation. DESIGN.md step 18 records the measured
  cost.

A surface feature names a Material or a Plant, exactly one, and a Plant feature
may name the Shape of its Voxels, overriding the Plant's own
([ADR-0027](0027-plants-are-declared-kinds-the-server-grows.md)). The snapshot
freezes what generation reads of the Plant, by Material and Shape name: its
Stages, a column's Material, height and fluid, a tree's Materials, size, fruit
and vines. A World therefore never changes when a Game later edits or drops the
Plant. A `climber` and a column growing down cannot be a feature.
