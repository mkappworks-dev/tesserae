# lumen

An enclosed garden of glass and light, whose lanterns light up one after
another.

## What it shows

Everything you can see comes from `glassworks`: clear `glass` walls and a
`frosted_glass` roof (both blended), `lantern` (emission 15), and the `slab`,
`stair` and `fence` Shapes. Ground is `strata`.

## Dependencies

[strata](../../mods/strata/), [glassworks](../../mods/glassworks/), and
[wanderer](../../mods/wanderer/) for its character. This Game declares no
Material, Shape or fluid of its own; every name it uses is `<mod id>:<name>`
from those Mods. It declares only its World (ground, sea level, relief and the
like) and one behaviour.

## Behaviour

On start the Game levels a 24 m square site 1 m above the ground band and builds
the garden on it: glass walls with a door in the side nearest the spawn, the
roof, a slab path with a stair at the door, and eight stone fence posts. Then,
every 2 s, the next lantern is placed on top of a post, going round the ring.
When all eight are lit they stay lit for 6 s, go out together, and the sequence
starts again.

The tests check that the build holds blended glass, an emitting Material and all
three Shapes, that no lantern is lit by the build itself, and that lanterns are
lit in order on the 2 s timer and put out together after the 6 s hold.

## Run it

Build the component, package the Game with the Mods it needs into one folder,
and point the Engine at it:

```sh
./scripts/build-guests
tesserae package mods/strata mods/glassworks mods/wanderer --out dist
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
