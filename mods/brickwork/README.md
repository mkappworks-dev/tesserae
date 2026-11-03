# brickwork

Masonry: brick, cobblestone and tile, with a wall Shape that grows arms towards
other walls.

| Declares  | Name                                                |
| --------- | --------------------------------------------------- |
| Materials | `brick`, `cobble`, `tile`                           |
| Shapes    | `wall` (a thick post with arms towards other walls) |

**Depends on:** nothing.

**Used by:** bog (brick is what mud and lava make).

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
draws them from code, so the artwork has no third-party source. Edit the script
and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/brickwork --mods mods
tesserae package mods/brickwork --out dist --mods mods
```

A Game lists it in `depends`. It declares no `world`, `view` or `tools`.
