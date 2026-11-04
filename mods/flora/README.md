# flora

Plants: four Materials. Data only; no code.

| Material | Render | Notes                                      |
| -------- | ------ | ------------------------------------------ |
| `wood`   | opaque | Trunks and timber; burns under `hearth`    |
| `leaves` | cutout | Light-passing canopy; burns under `hearth` |
| `grass`  | cutout | Tufts                                      |
| `kelp`   | cutout | Sea plant                                  |

Fuel is declared by whoever knows how it burns: `flora` names no combustion
rows, so a Game without `hearth` has plants that do not burn. Depends on
nothing.

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
