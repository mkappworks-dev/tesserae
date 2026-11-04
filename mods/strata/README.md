# strata

The ground: six solid Materials with 48 by 16 textures. Data only; no code.

| Material | Use                                                       |
| -------- | --------------------------------------------------------- |
| `stone`  | Rock under most land                                      |
| `basalt` | Dark rock; also what running lava cools to (see `hearth`) |
| `clay`   | Soil under meadows                                        |
| `sand`   | Beaches and sea floors                                    |
| `soil`   | Sub-surface earth                                         |
| `turf`   | Grass-topped surface                                      |

Each Material also has an item of the same name that a removed Voxel of it drops
(0.25 m). Declares no fluids, Shapes, reactions or fire. Depends on nothing.

**Used by:** `meadow`, `ember`, `tidewater`, `lumen`, and `hearth` (basalt).

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
in the repository draws them from code, so the artwork has no third-party
source. Edit the script and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/strata --mods mods
tesserae package mods/strata --out dist --mods mods
```

Copy `dist/mods/strata/` into the `mods/` folder beside `games/`. A Game lists
it in `depends`; this Mod declares no `world`, `view` or `tools`, which only a
Game may.
