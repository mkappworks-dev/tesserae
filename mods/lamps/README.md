# lamps

Light sources of three strengths and colours, and a small bulb Shape.

| Declares  | Name                                                  |
| --------- | ----------------------------------------------------- |
| Materials | `glowstone` (15), `amberlight` (12), `moonlight` (10) |
| Shapes    | `bulb` (a 6/16 cube)                                  |

**Depends on:** nothing.

**Used by:** nothing yet.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
draws them from code, so the artwork has no third-party source. Edit the script
and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/lamps --mods mods
tesserae package mods/lamps --out dist --mods mods
```

A Game lists it in `depends`. It declares no `world`, `view` or `tools`.
