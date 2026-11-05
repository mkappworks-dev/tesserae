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
