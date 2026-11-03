# bog

Wet ground: peat, a mud fluid and a tar fluid, and the rules that tie them to
lava and fire.

| Declares   | Name                                                                                            |
| ---------- | ----------------------------------------------------------------------------------------------- |
| Materials  | `peat`, `mud` and `tar` (fluids)                                                                |
| Fluids     | `bog:mud` (soaks into Shapes, puts out fire), `bog:tar` (slow, stays out of Shapes)             |
| Reactions  | mud + lava → `brickwork:brick`; tar + lava → `hearth:ash`; the fluid is consumed, the lava kept |
| Combustion | `bog:peat`: ignites at 4 after 3 s, burns 20 s, leaves `hearth:ash`                             |

**Depends on:** `hearth`, `brickwork` (and through `hearth`: `strata`, `waters`,
`flora`).

**Used by:** nothing yet.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
draws them from code, so the artwork has no third-party source. Edit the script
and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/bog --mods mods
tesserae package mods/bog --out dist --mods mods
```

A Game lists it in `depends`. It declares no `world`, `view` or `tools`.
