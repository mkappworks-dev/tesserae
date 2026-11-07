# lumen

An enclosed garden of glass and light, whose lanterns light up one after
another.

## What it shows

Everything you can see comes from `glassworks`: clear `glass` walls and a
`frosted_glass` roof (both blended), `lantern` (emission 15), and the `slab`,
`stair` and `fence` Shapes. Ground is `strata`, with `frost` snow in one court
and `crystals` amethyst in the rock below.

## Dependencies

[strata](../../mods/strata/), [glassworks](../../mods/glassworks/),
[frost](../../mods/frost/) for the snow of the frost court,
[crystals](../../mods/crystals/) for the amethyst ore, and it suggests
[wanderer](../../mods/wanderer/) for its character. Its one Material is its
food, `lumen:glowberry`; every other name it uses is `<mod id>:<name>` from
those Mods. It declares only its World (ground, sea level, relief, climate and
the like), its Biomes and ore, its food and one behaviour.

## Food

`lumen:glowberry`, worth 5 hunger and glowing like the lanterns, lies on the
surface of `garden` and `frost_court`, about one in every 20 by 20 m. Break it
to pick it up and press G to eat it.

## Biomes

Three Biomes, each with a sky and fog pair that eases as the Player walks
between them. The climate regions are 480 m across, far larger than the garden,
so the garden seldom straddles a border. The World has no caves, so no Biome
tunes them.

| Biome         | Ground (rock, soil, surface)      | Sky and fog |
| ------------- | --------------------------------- | ----------- |
| `garden`      | the Game's own: stone, soil, turf | soft green  |
| `arid_court`  | stone, sand, sand                 | warm sand   |
| `frost_court` | stone, soil, `frost:snow`         | cold blue   |

`crystals:amethyst` forms veins in the rock of every court, 8 to 40 m down.

## Behaviour

On start the Game levels a 24 m square site 1 m above the ground band and builds
the garden on it: glass walls with a door in the side nearest the spawn, the
roof, a slab path with a stair at the door, and eight stone fence posts. Then,
every 2 s, the next lantern is placed on top of a post, going round the ring.
When all eight are lit they stay lit for 6 s, go out together, and the sequence
starts again.

The tests check that the build holds blended glass, an emitting Material and all
three Shapes, that no lantern is lit by the build itself, and that lanterns are
lit in order on the 2 s timer and put out together after the 6 s hold. Others
check the three Biomes' sky and fog and ground, and that the amethyst lies in
rock within its band.

## Run it

Build the component, package the Game with the Mods it needs into one folder,
and point the Engine at it:

```sh
./scripts/build-guests
tesserae package mods/strata mods/glassworks mods/frost mods/crystals mods/wanderer --out dist
tesserae package games/lumen --mods mods --out dist --trust-for-run
tesserae --mods dist/mods
```

Then create a World with the Game `lumen` from the Games list. To check it
without opening a World:

```sh
tesserae validate games/lumen --mods mods --trust-for-run
```

From a checkout, replace `tesserae` with `cargo run --release --`.

## Licence

MIT, in `LICENSE`. The Mods it depends on are licensed by their own folders.
