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

Absent optional values are absent, as in the contract: an omitted `caves`,
`smoothing`, `water`, `ground`, `movement`, `character`, `view`, `heat`,
`collision`, `connect`, `product`, `receiving` or `residue` means none, never a
default. The one exception is a present `world` table's numeric fields
(`sea-level-m`, `relief-m`, `feature-size-m`, `soil-depth-m`, `octaves`), which
default to the Engine's terrain values. Every omitted list is empty. Field names
are the contract's, in kebab-case; unknown fields and variants at every level
fail naming the Mod. Values go through the same validation as a component
Declaration, so bounds, Game-only restrictions and per-Mod limits are identical.

| Only Games may declare `world`, `view` and `tools`. | Table | Fields |     |
| --------------------------------------------------- | ----- | ------ | --- |

---

| | `[[shapes]]` | `name`, `boxes`, optional `collision`, optional `connect` | |
`boxes` entry | `min`, `max`: three integers, sixteenths of a Voxel (collision
boxes are signed) | | `[shapes.connect]` | `group`, `connects-to`, `to-sealed`,
`arms` (`face`, `boxes`, optional `collision`) | | `[view]` | `view-distance-m`,
`lod-distance-m` | | `[world.caves]` | `feature-size-m`, `radius-m`, `floor-m`,
`octaves` | | `[world.smoothing]` | `min-rise-m` | | `[world.water]` |
`level-m`, `material` (a fluid, by full name) | | `[world.movement]` |
`allow-flight`, `allow-physics`, `start-in-flight` | | `[world.character]` |
`bones`, `pixel-m`, `skin` (a 64 by 64 PNG path), `idle`, `walk`, `sprint`,
`airborne`, `crouch` (limb motion: `swing-deg`, `rate-hz`, `arm-raise-deg`) | |
character bone | `name`, optional `parent`, `role` (`body`, `head`, `arm-left`,
`arm-right`, `leg-left`, `leg-right`, `fixed`), `pivot`, `cubes` (`origin`,
`size`, `uv`) | | `[[fluids]]` | `material`, `reach-m`, `sideways-m-per-s`,
`downward-m-per-s`, `recession-m-per-s`, `reset-drop-m`, `waterlogs`,
`extinguishes`, optional `heat` (`strength`, `reach-m`) | | `[[reactions]]` |
`first`, `second` (`fluid`, `state`: `any`, `source` or `flowing`), `consumed`
(`first` or `second`), optional `product`, optional `receiving` | |
`[[combustion]]` | `fuel`, `ignition-threshold`, `ignition-delay-s`, `emission`,
optional `heat`, `end` | | `[[tools]]` | `name`, `kind` (`measure`) |

A combustion `end` is `"permanent"` or
`{ after = { seconds = 8, residue = "mod:ash" } }`; `residue` omitted removes
the solid. Fluid, reaction, combustion, ground and water names are full
`<mod id>:<name>` names and resolve after every Mod has declared, in dependency
order, whether their Material came from data or a component. Data-only Mods have
no guest instances, `start` or `tick` callbacks. Dependencies use the existing
`depends` Manifest field and load before their consumers, regardless of
Declaration format. Paths must be relative and stay inside the installed
directory after symlink resolution.
