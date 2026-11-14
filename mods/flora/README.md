# flora

Plants: placed Materials, and Plants the Server grows. Data only; no code.

| Material | Render | Notes                                      |
| -------- | ------ | ------------------------------------------ |
| `wood`   | opaque | Trunks and timber; burns under `hearth`    |
| `leaves` | cutout | Light-passing canopy; burns under `hearth` |
| `grass`  | cutout | Tufts                                      |
| `kelp`   | cutout | Sea plant                                  |

Fuel is declared by whoever knows how it burns: `flora` names no combustion
rows, so a Game without `hearth` has plants that do not burn. Depends on
`strata`, whose soils its Plants grow on.

The oak is a `tree` of the grown Materials `oak_log` and `oak_leaves`, which
look like `wood` and `leaves` but have no Item, so they cannot be placed:
breaking a trunk drops the placed `log`. `tuft`, `poppy` and `daisy` are
`spreader`s drawn with the `cross` Shape. `frond` is a kelp Material only: a
column that stands in water must name a fluid `flora` does not declare, so the
Game that has one, such as `tidewater`, declares the Plant.

**Used by:** `meadow`, `ember`, `tidewater`, and `hearth` (fuel).

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
