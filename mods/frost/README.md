# frost

Cold content: clear ice, snow, packed ice, a slow slush fluid and a snow layer
Shape.

| Declares  | Name                                                   |
| --------- | ------------------------------------------------------ |
| Materials | `ice` (blended), `snow`, `packed_ice`, `slush` (fluid) |
| Shapes    | `snow_layer` (a 4/16 slab)                             |
| Fluids    | `frost:slush`: reach 2 m, slow, puts out fire          |

**Depends on:** nothing.

**Used by:** nothing yet.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
draws them from code, so the artwork has no third-party source. Edit the script
and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/frost --mods mods
tesserae package mods/frost --out dist --mods mods
```

A Game lists it in `depends`. It declares no `world`, `view` or `tools`.
