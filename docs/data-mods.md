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

An appearance row may add `texture = "art/rock.png"`: a PNG inside the Mod
directory, exactly 48 by 16 pixels (three 16 by 16 faces), used for Material
surfaces and item previews. A Game's `[world.character]` takes `pixel-m`,
`skin = "skin.png"` (64 by 64), `idle`, `walk`, `sprint`, `airborne` and
`crouch` limb motions (`swing-deg`, `rate-hz`, `arm-raise-deg`) and
`[[world.character.bones]]` rows (`name`, optional `parent`, `role`, `pivot`,
`cubes` of `origin`, `size`, `uv`) under the same limits as the WIT character.
PNGs of any colour type decode to straight-alpha RGBA. A missing, invalid,
wrongly sized or escaping file fails before any World opens, naming the Mod and
field.

Referenced artwork joins build identity with the Manifest and Declaration, and
the bytes that were checked are the bytes decoded. Changing a referenced PNG
needs Trust again and breaks exact-build reopening of saved Worlds; unreferenced
PNGs and editor files do not.

Only Games may declare `world`. Its optional numeric fields are `sea-level-m`,
`relief-m`, `feature-size-m`, `soil-depth-m` and `octaves`, defaulting to the
Engine's terrain values. Omitted ground means empty air; caves, smoothing and
water are absent in this initial schema. Unknown fields at every level fail
explicitly. Shapes, view, movement, characters, simulation and Tools are added
by subsequent SDK tickets, not silently ignored here.

Data-only Mods have no guest instances, `start` or `tick` callbacks.
Dependencies use the existing `depends` Manifest field and load before their
consumers, regardless of Declaration format. Paths must be relative and stay
inside the installed directory after symlink resolution.
