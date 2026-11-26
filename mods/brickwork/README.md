# brickwork

Masonry: cobble, dressed stone, slate, brick, roof tile and whitewash, made from
`strata`'s stone, clay and sand at `workshop`'s Stations, with a wall Shape that
grows arms towards other walls.

| Declares  | Name                                                                  |
| --------- | --------------------------------------------------------------------- |
| Materials | `cobble`, `ashlar`, `slate`, `brick`, `tile`, `red_tile`, `whitewash` |
| Shapes    | `wall` (a thick post with arms towards other walls)                   |

| Recipe        | From                              | At    |
| ------------- | --------------------------------- | ----- |
| `cobble`      | 1 `strata:stone`                  | hand  |
| 2 `ashlar`    | 2 `strata:stone`                  | anvil |
| 3 `slate`     | 2 `strata:stone`                  | anvil |
| 2 `brick`     | 2 `strata:clay`                   | oven  |
| 4 `tile`      | 1 `strata:clay`, 1 `strata:sand`  | oven  |
| 4 `red_tile`  | 2 `strata:clay`                   | oven  |
| 2 `whitewash` | 1 `strata:sand`, 1 `strata:stone` | oven  |

**Depends on:** `strata`, `workshop`.

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
