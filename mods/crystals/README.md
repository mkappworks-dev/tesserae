# crystals

Glowing, see-through gems and a cluster Shape.

| Declares  | Name                                                           |
| --------- | -------------------------------------------------------------- |
| Materials | `amethyst`, `citrine`, `aquamarine` (blended, emission 6 to 7) |
| Shapes    | `cluster` (a base with a spike)                                |

**Depends on:** nothing.

**Used by:** nothing yet.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
draws them from code, so the artwork has no third-party source. Edit the script
and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/crystals --mods mods
tesserae package mods/crystals --out dist --mods mods
```

A Game lists it in `depends`. It declares no `world`, `view` or `tools`.
