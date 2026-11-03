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
[flora](../../mods/flora/), [hearth](../../mods/hearth/), and
[golem](../../mods/golem/) for its character. This Game declares no Material,
Shape or fluid of its own; every name it uses is `<mod id>:<name>` from those
Mods. It declares only its World (ground, sea level, relief and the like) and
one behaviour.

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
wood-to-ash fire.

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
