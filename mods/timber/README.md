# timber

Building timber: sawn planks and thatch, with a horizontal beam and a thin
upright panel.

| Declares  | Name                                                |
| --------- | --------------------------------------------------- |
| Materials | `planks`, `thatch`                                  |
| Shapes    | `beam` (a bar along x), `panel` (a 2/16 thick wall) |

**Depends on:** nothing.

**Used by:** nothing yet.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
draws them from code, so the artwork has no third-party source. Edit the script
and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/timber --mods mods
tesserae package mods/timber --out dist --mods mods
```

A Game lists it in `depends`. It declares no `world`, `view` or `tools`.
