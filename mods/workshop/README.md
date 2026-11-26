# workshop

The Stations a building's Materials are made at, and a chest.

| Declares   | Name                                                                                    |
| ---------- | --------------------------------------------------------------------------------------- |
| Materials  | `workbench`, `forge` (glows), `anvil`, `oven`, `chest`                                  |
| Shapes     | `table` (four legs under a thick top), which the workbench takes                        |
| Voxel uses | a Station of each of `workbench`, `forge`, `anvil` and `oven`; the chest is a Container |

| Recipe    | From                              | At        |
| --------- | --------------------------------- | --------- |
| workbench | 4 `flora:log`                     | hand      |
| forge     | 8 `strata:stone`                  | hand      |
| oven      | 4 `strata:stone`, 2 `strata:sand` | hand      |
| anvil     | 6 `strata:stone`                  | forge     |
| chest     | 8 `flora:log`                     | workbench |

A Station's tag is shared by every Station that names it, so a Game's own bench
under the tag `workbench` opens the same Recipes as this one.

**Depends on:** `flora`, `strata`.

**Used by:** `timber`, `brickwork`, `glassworks`.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
draws them from code, so the artwork has no third-party source. Edit the script
and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/workshop --mods mods
tesserae package mods/workshop --out dist --mods mods
```

A Game lists it in `depends`. It declares no `world`, `view` or `tools`.
