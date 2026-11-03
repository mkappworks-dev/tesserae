# wool

Six coloured cloths and a carpet Shape.

| Declares  | Name                                                                             |
| --------- | -------------------------------------------------------------------------------- |
| Materials | `wool_red`, `wool_blue`, `wool_green`, `wool_yellow`, `wool_white`, `wool_black` |
| Shapes    | `carpet` (a 2/16 slab)                                                           |

**Depends on:** nothing.

**Used by:** nothing yet.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
draws them from code, so the artwork has no third-party source. Edit the script
and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/wool --mods mods
tesserae package mods/wool --out dist --mods mods
```

A Game lists it in `depends`. It declares no `world`, `view` or `tools`.
