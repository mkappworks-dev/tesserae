# Data-only Mods (contract 0.3)

Install Games under `games/<id>/` and content Mods under `mods/<id>/`. Select
exactly one source in `mod.toml`: existing `wasm = "game.wasm"` or
`declaration = "content.toml"`. Data Declarations require `api = "0.3"`. Trust
authorizes the exact Manifest and Declaration bytes. Saved Worlds require that
exact build again; unrelated files do not affect identity.

A minimal Game's `mod.toml`:

```toml
id = "plain"
version = "0.1.0"
api = "0.3"
root = true
declaration = "content.toml"
```

Its `content.toml`:

```toml
[[materials]]
name = "rock"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[appearance]]
material = "rock"
color = [80, 90, 100, 255]

[world.ground]
rock = "plain:rock"
soil = "plain:rock"
surface = "plain:rock"
```

Material rows require the six fields above and accept optional `remove-time-s`.
`occlusion` is `opaque`, `same-material` or `never`; `render` is `opaque`,
`blend` or `cutout`. Appearance colours are four RGBA bytes; Material names in
appearance are local, while ground names include the Mod id. Both formats use
the same content bounds and validation.

Only Games may declare `world`. Its optional numeric fields are `sea-level-m`,
`relief-m`, `feature-size-m`, `soil-depth-m` and `octaves`, defaulting to the
Engine's terrain values. Omitted ground means empty air; caves, smoothing and
water are absent in this initial schema. Unknown fields at every level fail
explicitly. Shapes, textures, view, movement, characters, simulation and Tools
are added by subsequent SDK tickets, not silently ignored here.

Data-only Mods have no guest instances, `start` or `tick` callbacks.
Dependencies use the existing `depends` Manifest field and load before their
consumers, regardless of Declaration format. Paths must be relative and stay
inside the installed directory after symlink resolution.
