# crops

Farmed vegetables and fruit, with their food Items and Recipes. Data only; no
code. Depends on `strata` (the `#soil` tag) and `flora` (the `cross` Shape).

| Plant          | Stages                            | Gives                  |
| -------------- | --------------------------------- | ---------------------- |
| `crops:wheat`  | sprout, young, green, ripe        | `wheat`, `wheat_seeds` |
| `crops:carrot` | sprout, young, green, ripe        | `carrot` (food), seeds |
| `crops:potato` | sprout, young, green, ripe        | `potato` (food), seeds |
| `crops:berry`  | sprout, bush, ripe (fruits again) | `berries` (food)       |

Each crop is planted from its seed Item (`berry_cutting` for the bush) and needs
light 9 or more. Every Stage and Item has artwork drawn by
`scripts/generate-mod-art`.

Recipes, both by hand: three `wheat` make a `bread`; two `berries` and a `wheat`
make a `berry_pie`.

**Used by:** `terra`, `meadow`, `ember`, `tidewater` and `lumen`, which scatter
them in Biomes.

## Licence and artwork

MIT, in `LICENSE`; the PNGs in `art/` are drawn by `scripts/generate-mod-art`.

## Use it

```sh
tesserae validate mods/crops --mods mods
tesserae package mods/crops --out dist --mods mods
```
