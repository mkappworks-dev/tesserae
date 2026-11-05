# meadow

Gentle rolling land with a glade of trees and a glass-roofed gazebo.

## What it shows

A small Rust Game that gets all of its content from data Mods: ground from
`strata`, trees from `flora`, and the `fence` and `slab` Shapes and the glass
from `glassworks`. A Shape from one Mod can carry a Material from another: the
gazebo posts are `flora:wood` in the `glassworks:fence` Shape.

## Dependencies

[strata](../../mods/strata/), [flora](../../mods/flora/),
[glassworks](../../mods/glassworks/), and it suggests
[wanderer](../../mods/wanderer/) for its character. Its one Material is its
food, `meadow:pumpkin`; every other name it uses is `<mod id>:<name>` from those
Mods. It declares only its World (ground, sea level, relief, climate and the
like), its Biomes and ore, its food and one behaviour.

## Food

`meadow:pumpkin`, worth 8 hunger, lies on the surface of `meadow` and `heath`,
about one in every 20 by 20 m. Break it to pick it up and press G to eat it.

## Biomes

Three Biomes, each with a sky and fog pair that eases as the Player walks
between them. The climate regions are 400 m across.

| Biome    | Ground (rock, soil, surface)      | Sky and fog   |
| -------- | --------------------------------- | ------------- |
| `meadow` | the Game's own: stone, clay, turf | clear blue    |
| `heath`  | stone, clay, `strata:soil`        | overcast grey |
| `dunes`  | stone, sand, sand                 | warm sand     |

`strata:basalt` forms dark veins in the rock of every Biome, 6 to 36 m down. The
World has no caves, so no Biome tunes them.

## Behaviour

On start (not on a reopened World) the Game queues one build and sends it over
the first ticks, in batches within the host's per-batch cap: a levelled turf
glade 16 m square, three trees (a trunk and a two-part crown each), and a gazebo
of four posts under a glass slab roof. The glade is cut at a fixed height in the
middle of the ground band, because a Mod cannot read the terrain it will build
on.

The test builds the Game, loads it with its three Mods and checks that no
Material is declared in Rust, that the ground names `strata` Materials, and that
the trees and gazebo are placed at 0.5 m and 0.25 m Voxels. Others check the
three Biomes' sky and fog and ground, and that the basalt veins lie in rock
within their band.

## Run it

Build the component, package the Game with the Mods it needs into one folder,
and point the Engine at it:

```sh
./scripts/build-guests
tesserae package mods/strata mods/flora mods/glassworks mods/wanderer --out dist
tesserae package games/meadow --mods mods --out dist --trust-for-run
tesserae --mods dist/mods
```

Then create a World with the Game `meadow` from the Games list. To check it
without opening a World:

```sh
tesserae validate games/meadow --mods mods --trust-for-run
```

From a checkout, replace `tesserae` with `cargo run --release --`.

## Licence

MIT, in `LICENSE`. The Mods it depends on are licensed by their own folders.
