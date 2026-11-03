# waters

Water as content: the Engine ships none (ADR-0016).

| Declares  | Name                                                           |
| --------- | -------------------------------------------------------------- |
| Materials | `water` (fluid), `steam` (decoration, not a fluid), `obsidian` |
| Fluids    | `waters:water`: reach 7 m, soaks into Shapes and puts out fire |

`obsidian` is here so that a Game with water and `hearth` can cool lava to it;
the reaction that makes it is `hearth`'s, because it needs lava and this Mod
must work where there is none. Depends on nothing.

**Used by:** `ember`, `tidewater`, and `hearth` (reactions).

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
in the repository draws them from code, so the artwork has no third-party
source. Edit the script and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/waters --mods mods
tesserae package mods/waters --out dist --mods mods
```

Copy `dist/mods/waters/` into the `mods/` folder beside `games/`. A Game lists
it in `depends`; this Mod declares no `world`, `view` or `tools`, which only a
Game may.
