# aquatic

Water plants. Data only; no code. Depends on `waters` for a fluid tagged `water`
and on `strata` for the seabed tags, and uses `flora`'s `cross` Shape.

| Plant              | Max height | Notes                              |
| ------------------ | ---------- | ---------------------------------- |
| `aquatic:kelp`     | 5 m        | A `column` that stands in `#water` |
| `aquatic:seagrass` | 1 m        | Short, on the same terms           |
| `aquatic:red_weed` | 2 m        | Red                                |

They stand in any fluid tagged `water`, so a Game with its own water, such as
`terra`, tags it and does not need `waters`' fluid. The `lily_pad` is a
decorative Material with a flat `pad` Shape that a Biome places by feature, not
a Plant: a Plant's soil must be solid, and a Plant cannot yet grow on a fluid.

**Used by:** `terra`.

## Licence and artwork

MIT, in `LICENSE`; the PNGs in `art/` are drawn by `scripts/generate-mod-art`.

## Use it

```sh
tesserae validate mods/aquatic --mods mods
tesserae package mods/aquatic --out dist --mods mods
```
