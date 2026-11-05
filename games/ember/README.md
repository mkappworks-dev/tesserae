# ember

A volcanic highland: basalt chimneys that erupt lava on a timer, over
ash-covered ground.

## What it shows

The rules that make a volcano are all in the Mods. Lava is a hot fluid
(`hearth`), water flows (`waters`), and where they meet `hearth` turns lava that
is standing into `waters:obsidian` and lava that is running into
`strata:basalt`. Wood (`flora`) burns and leaves `hearth:ash`. The Game only
sets the scene and starts the eruptions.

## Dependencies

[strata](../../mods/strata/), [waters](../../mods/waters/),
[flora](../../mods/flora/), [hearth](../../mods/hearth/), and it suggests
[golem](../../mods/golem/) for its character. Its one Material is its food,
`ember:pepper`; every other name it uses is `<mod id>:<name>` from those Mods.
It declares only its World (ground, sea level, relief, climate and the like),
its Biomes and ore, its food and one behaviour.

## Food

`ember:pepper`, a fire pepper worth 4 hunger, lies on the surface of `ash_flats`
and `highland`, about one in every 16 by 16 m. Break it to pick it up and press
G to eat it.

## Biomes

Three Biomes, each with a sky and fog pair that eases as the Player walks
between them. The climate regions are 320 m across.

| Biome       | Ground (rock, soil, surface)                | Caves  | Sky and fog         |
| ----------- | ------------------------------------------- | ------ | ------------------- |
| `ash_flats` | the Game's own: basalt, stone, `hearth:ash` | as set | grey, a little warm |
| `scorched`  | basalt, basalt, `strata:clay`               | 1.5 x  | rust red            |
| `highland`  | stone, soil, `flora:grass`                  | 0.5 x  | cool blue-grey      |

`waters:obsidian` also forms veins in the rock of the ash flats and the scorched
ground, 6 to 48 m down, and nowhere in the highland. Every name is from a Mod
the Game already depends on.

## Behaviour

Three 3 m chimneys are built on start. Each vent erupts (a lava source on its
cap) for 8 s out of every 30 s, and the vents start 10 s apart, the first 5 s
after the build is applied; at the end of an eruption the source is removed and
the flow recedes. The first chimney has a wooden deck its lava runs onto; wood
burns under `hearth`'s rules. The third has a standing water source on its cap,
2 m from its lava. Crater lakes in the valleys are `waters:water`.

The timer is in simulation seconds, so eruptions are the same at any Voxel size.
The tests observe each vent start and stop on that schedule, and check that the
loaded tables hold the lava and water fluids, the obsidian reaction and the
wood-to-ash fire. Others check the three Biomes' sky and fog and ground, and
that the ore lies in rock within its band and Biomes.

## Run it

Build the component, package the Game with the Mods it needs into one folder,
and point the Engine at it:

```sh
./scripts/build-guests
tesserae package mods/strata mods/waters mods/flora mods/hearth mods/golem --out dist
tesserae package games/ember --mods mods --out dist --trust-for-run
tesserae --mods dist/mods
```

Then create a World with the Game `ember` from the Games list. To check it
without opening a World:

```sh
tesserae validate games/ember --mods mods --trust-for-run
```

From a checkout, replace `tesserae` with `cargo run --release --`.

## Licence

MIT, in `LICENSE`. The Mods it depends on are licensed by their own folders.
