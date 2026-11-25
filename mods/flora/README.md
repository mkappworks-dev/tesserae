# flora

Trees, fruit, ground cover, flowers, mushrooms, cacti and vines: placed
Materials, and Plants the Server grows. Data only; no code. Its soils are tags
(`#soil`, `#sand`, `#rock`), so any Game whose ground carries them grows these
Plants; `strata` tags its own, and a Game with its own ground tags that.

| Material | Render | Notes                                      |
| -------- | ------ | ------------------------------------------ |
| `wood`   | opaque | Trunks and timber; burns under `hearth`    |
| `leaves` | cutout | Light-passing canopy; burns under `hearth` |
| `grass`  | cutout | Tufts                                      |
| `kelp`   | cutout | Sea plant                                  |

Fuel is declared by whoever knows how it burns: `flora` names no combustion
rows, so a Game without `hearth` has plants that do not burn. Depends on
`strata`.

## Trees

Each `tree` Plant has its own grown trunk, canopy and sapling Materials. The
trunk and canopy have no Item, so nobody can place them: felling drops the
placed `log` and `leaves`, and a sapling Item plants the next tree. Sizes are in
metres.

| Plant          | Trunk m    | Canopy radius m | Notes                       |
| -------------- | ---------- | --------------- | --------------------------- |
| `flora:oak`    | 8 to 14    | 2.5             |                             |
| `flora:birch`  | 5 to 7.5   | 1.5             | Tall, thin, pale trunk      |
| `flora:pine`   | 7 to 10    | 1.2             | Narrow; see below           |
| `flora:palm`   | 6 to 9     | 1.8             | Coconuts; sand or soil      |
| `flora:acacia` | 3.5 to 5.5 | 3.5             | Wide crown                  |
| `flora:jungle` | 8 to 27    | 3 to 6          | Storeys; hangs `flora:vine` |
| `flora:swamp`  | 6 to 15    | 2.5 to 3.5      | Storeys; hangs `flora:vine` |
| `flora:apple`  | 4 to 6     | 2.5             | Apples, a 3D box Shape      |
| `flora:orange` | 3 to 4.5   | 2.2             | Oranges                     |
| `flora:cherry` | 4 to 6     | 2.5             | Cherries                    |

A tree's fruit ripens through two Stages (green, then red, orange, dark red or
brown), is picked with Use and regrows. The food Items are `apple`, `orange`,
`cherries` and `coconut`.

Every species branches: the oak forks into leaders and spreads limbs, the birch
and swamp tree lean, the pine has tiers of drooping limbs that shorten upward (a
cone), the palm leans under its crown, the acacia forks low into a flat crown of
lobes, the jungle tree spreads wide limbs, and the apple, orange and cherry have
low spreading limbs with fruit on the lobes. The crown keeps `canopy-radius-m`;
buttresses and curved limbs wait for Structures.

The jungle tree grows in three storeys (15% emergent at 25 to 27 m under a 6 m
crown, 60% at 18 to 24 m under 4.5 m, 25% understory at 8 to 12 m under 3 m) and
the swamp tree in two (70% at 12 to 15 m, 30% at 6 to 9 m under 2.5 m), so a
forest has crowns above and below its canopy. The jungle, oak, swamp, apple,
orange and cherry trees each grow one of three canopy variants, the leaves and a
`_dark` and `_light` copy about 8% darker or lighter and 4 degrees apart in hue,
so neighbouring crowns read as separate trees. Picked fruit leaves the first.

`birch_wood`, `pine_wood`, `palm_wood`, `acacia_wood` and `jungle_wood` are
placed look-alikes for builders, made from a `log` by hand.

## Ground cover and others

`spreader`s drawn with the `cross` Shape: `tuft`, `tall_grass`, `dry_grass`,
`fern`, `poppy`, `daisy`, `dandelion`, `orchid` and the mushrooms `mushroom_red`
and `mushroom_brown`, which need low light. Each sets `min-neighbours`, so a
patch that worldgen placed fills its holes and does not advance, while a single
planted seed still grows one neighbour; plant a handful to start a patch. `cane`
and `cactus` are `column`s up; `cane` needs no water, since `flora` cannot name
a fluid. The `cactus` is a boxed Shape that blocks and hurts on contact. `vine`
is a `climber` drawn with the `plane` Shape; it grows on rock, soil, placed wood
and leaves, and on the jungle and swamp trees, whose worldgen hangs it.

`frond` is a kelp Material only; the Plants that stand in water are in
[`aquatic`](../aquatic/).

**Used by:** `meadow`, `ember`, `tidewater`, `lumen`, `terra` and `crops`, and
`hearth` (fuel).

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
in the repository draws them from code, so the artwork has no third-party
source. Edit the script and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/flora --mods mods
tesserae package mods/flora --out dist --mods mods
```

Copy `dist/mods/flora/` into the `mods/` folder beside `games/`. A Game lists it
in `depends`; this Mod declares no `world`, `view` or `tools`, which only a Game
may.

The `wood` Material also has a `log` item (0.3 m) that a removed Voxel of it
drops.
