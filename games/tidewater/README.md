# tidewater

Islands and shallow seas whose sea level rises and falls, slowly.

## What it shows

World parameters: `sea-level-m`, `relief-m`, a `caves` table for sea caves, a
`smoothing` table that eases one-Voxel steps into stairs and slabs, and a
standing `water` level of `waters:water` that makes islands of the high ground.
The tide itself is Commands, not terrain: layers of water sources added and
removed above the sea.

## Dependencies

[strata](../../mods/strata/), [waters](../../mods/waters/),
[flora](../../mods/flora/), and it suggests [sprite](../../mods/sprite/) for its
character. This Game declares no Material, Shape or fluid of its own; every name
it uses is `<mod id>:<name>` from those Mods. It declares only its World
(ground, sea level, relief and the like) and one behaviour.

## Behaviour

High tide is 1 m above the sea (26 m). Every 8 s the Game adds one Voxel-thick
layer of sources on the way up, takes one away on the way down, and turns round
at each end; at 0.5 m Voxels that is a 48 s cycle. The tide acts on a 48 m
square round the origin, in strips within the fill cap, queued over as many
ticks as the batch cap needs. A reopened World starts by taking every layer
away. Six kelp stalks (`flora:kelp`) are planted at fixed spots from below the
sea floor band to just under the surface.

The Player spawns 2 m above the higher of the ground and the sea, and the Game
asserts at compile time that the tide range is smaller than that. The tests
check that every Command stays between the sea and high tide at 0.5, 0.25 and
0.125 m Voxels, that the tide reaches its full range and falls again, and that
the real generator's spawn is above high tide for 32 seeds.

## Run it

Build the component, package the Game with the Mods it needs into one folder,
and point the Engine at it:

```sh
./scripts/build-guests
tesserae package mods/strata mods/waters mods/flora mods/sprite --out dist
tesserae package games/tidewater --mods mods --out dist --trust-for-run
tesserae --mods dist/mods
```

Then create a World with the Game `tidewater` from the Games list. To check it
without opening a World:

```sh
tesserae validate games/tidewater --mods mods --trust-for-run
```

From a checkout, replace `tesserae` with `cargo run --release --`.

## Licence

MIT, in `LICENSE`. The Mods it depends on are licensed by their own folders.
