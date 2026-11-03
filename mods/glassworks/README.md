# glassworks

Glass, light, and the Shapes for building with them.

| Declares  | Name                                                                                 |
| --------- | ------------------------------------------------------------------------------------ |
| Materials | `glass` (clear, blended), `frosted_glass` (blended, dimmer), `lantern` (emission 15) |
| Shapes    | `slab` (half height), `stair`, `fence` (a post that grows arms towards other fences) |

Depends on nothing.

**Used by:** `meadow`, `lumen`.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
in the repository draws them from code, so the artwork has no third-party
source. Edit the script and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/glassworks --mods mods
tesserae package mods/glassworks --out dist --mods mods
```

Copy `dist/mods/glassworks/` into the `mods/` folder beside `games/`. A Game
lists it in `depends`; this Mod declares no `world`, `view` or `tools`, which
only a Game may.
