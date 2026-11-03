# hearth

Heat: lava, ash, and the rules that join them to water and wood.

| Declares   | Name                                                                                                                  |
| ---------- | --------------------------------------------------------------------------------------------------------------------- |
| Materials  | `lava` (fluid, emission 15), `ash`                                                                                    |
| Fluids     | `hearth:lava`: reach 3 m, hot (strength 12 over 2 m), stays out of Shapes                                             |
| Reactions  | lava source + water → `waters:obsidian`; flowing lava + water → `strata:basalt`; the lava is consumed, the water kept |
| Combustion | `flora:wood`: ignites at 6 after 2 s, burns 8 s, leaves `hearth:ash`; `flora:leaves`: flares for 3 s, leaves nothing  |

**Depends on:** `strata`, `waters`, `flora`. The reactions and fuel name their
Materials, so they load first; remove one and `tesserae validate` names the
missing Mod.

**Used by:** `ember`.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
in the repository draws them from code, so the artwork has no third-party
source. Edit the script and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/hearth --mods mods
tesserae package mods/hearth --out dist --mods mods
```

Copy `dist/mods/hearth/` into the `mods/` folder beside `games/`. A Game lists
it in `depends`; this Mod declares no `world`, `view` or `tools`, which only a
Game may.
