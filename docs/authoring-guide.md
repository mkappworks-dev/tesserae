# Authoring guide

How to make a Game or a Mod for Tesserae, from a starter to an installed folder.
Nothing here needs the Engine's source tree: the two [starters](../starters/)
are copied anywhere and refer to nothing in this repository.

`tesserae` below is the Engine binary. From a checkout, run
`cargo run --release --` in its place, or call `target/release/tesserae`. Every
command in this guide was run against the starters copied to a folder outside
the checkout; see [What has been checked](#what-has-been-checked).

## Choose a starter

| Starter                                         | You write                     | Needs                    | Contract line |
| ----------------------------------------------- | ----------------------------- | ------------------------ | ------------- |
| [`starters/data-only/`](../starters/data-only/) | TOML and PNG files            | Nothing but the Engine   | 0.3           |
| [`starters/rust/`](../starters/rust/)           | Rust, compiled to WebAssembly | Rust and `wasm32-wasip2` | 0.3           |

Both produce the same installed folder and load through the same code. A Game
may be either; a Mod may be either; a World may mix them (see
[Mixing data and code](#mixing-data-and-code)). A single Mod has exactly one
source: data or code, never both.

```sh
cp -R path/to/tesserae/starters/data-only ~/work/my_game
cd ~/work/my_game
```

Rename the `id` in `mod.toml` (`[a-z][a-z0-9_]{0,31}`) and every `starter_data:`
in `content.toml` to match. The Rust starter lists its own renames in its
README.

## The data-only path

```sh
tesserae validate .
tesserae dev .
tesserae package . --out dist
```

1. `validate` loads the project through the code a World uses and prints
   `ok: game ...`, or exits 1 naming the Mod and the field. It opens no World.
2. `dev` opens a disposable World and restarts it whenever you save a valid
   change to `mod.toml`, `content.toml` or a PNG they reference. See
   [The development loop](#the-development-loop).
3. `package` writes `dist/games/<id>/` holding exactly `mod.toml`,
   `content.toml` and the referenced PNGs.

Install by copying `dist/games/<id>/` into the Engine's `games/` folder beside
`mods/`, or point the Engine at the output with `--mods dist/mods`. The first
run asks you to trust the build (see [Trust](#trust-and-exact-builds)); create a
World with the Game.

## The Rust path

```sh
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 --release
cp target/wasm32-wasip2/release/starter_rust.wasm .
tesserae validate . --trust-for-run
tesserae package . --out dist --trust-for-run
```

- The starter's only dependency is `wit-bindgen`; it builds against the bundled
  `wit/` copy of the 0.3 contract, not an Engine crate.
- A component runs code, so checking it needs a recorded Trust grant or
  `--trust-for-run`, which applies to the projects you name and writes nothing.
  Without it, `validate` stops with
  ``mod `starter_rust` is not trusted; pass --trust-for-run``. Data Declarations
  run no code and need neither.
- `package` does not run Cargo; build first.

To rebuild on every edit during `tesserae dev`, opt in with a `dev.toml` beside
`mod.toml`:

```toml
[build]
enabled = true
command = ["cargo", "build", "--target", "wasm32-wasip2", "--release"]
output = "target/wasm32-wasip2/release/starter_rust.wasm"
```

## Mixing data and code

A World chooses one Game and any Mods, and freezes that choice with their
dependencies. The two kinds depend on each other through the ordinary `depends`
field. A data Mod on the Rust Game:

```toml
id = "extra"
version = "0.1.0"
api = "0.3"
depends = ["starter_rust"]
declaration = "content.toml"
```

Name both projects to validate, package or watch them together; a project
shadows an installed item of its id, and a dependency the projects do not
include is read from `--mods`:

```sh
tesserae validate my_rust_game extra --trust-for-run
tesserae package my_rust_game extra --out dist --trust-for-run
tesserae dev my_rust_game extra
```

The result is `dist/games/starter_rust/` and `dist/mods/extra/`. Validated
alone, `extra` fails with ``depends on `starter_rust`, which is not installed``
unless `--mods` supplies it. Dependencies load first, whichever source they use,
and names resolve across the whole set.

The repository's [demo Games and shared Mods](demo-games.md) are worked examples
of this: four small Rust Games that depend on shared data Mods.

## One Declaration source per Mod

`mod.toml` selects exactly one of `wasm = "game.wasm"` or
`declaration = "content.toml"`. Neither, or both, is refused:
`select exactly one source: wasm or declaration`. A Declaration needs
`api = "0.3"`; on an older line it is refused with
`declaration requires api 0.3`. Older components keep loading on their own
lines.

A data Declaration is the same content a component returns from `declare`, and
goes through the same validation, bounds and per-Mod limits. It has no `start`
or `tick`, and so cannot place Voxels at run time.

## The Declaration schema

Field names are the contract's, in kebab-case. Unknown fields and variants fail
at every level, naming the Mod. Absent optional tables mean _none_, never a
default; the single exception is the numeric fields of a present `[world]`,
which take the Engine's terrain values. Omitted lists are empty.

Only a Game may declare `world`, `view`, `tools` and `cause-sounds`, except that
a Mod may supply `world.character` alone
([ADR-0021](adr/0021-a-mod-may-supply-the-character.md)). Any Mod may add
`[[biomes]]` and `[[ores]]` to the Game's World, in load order, a Biome's id
being its place in that order; only the Game declares `[world.climate]`. A Biome
point is in the 0 to 1 Climate square, an ore band is metres below the surface,
and an ore without `biomes` is in every Biome. A Biome may name its own `ground`
(rock, soil and surface, as in `[world.ground]`); a column takes the ground of
its Biome and switches hard at the border, and a Biome without one has the
World's. A Biome never changes the surface height. A Biome may scale the caves
with `cave-scale` (0 to 4, a multiplier on `caves.radius_m`; 0 is no caves),
which eases across the border where `ground` switches. A Biome may also name a
`sky-fog` pair of sRGB colours (see [Data Mods](data-mods.md)); the sky and fog
ease to the blend of the Biomes around the Player. A Biome may name `selectors`,
ranges of `altitude` (metres above Sea level), `coast` (metres inland),
`erosion` (0 rugged to 1 worn) and `river` (metres a river's valley cuts the
ground), and a `relief` scale (0 to 4) for its landform's detail; a Biome that
names a range is preferred where the column fits it. It may also bed up to 8
`strata`, each a solid `material` from `depth-min-m` to `depth-max-m` below the
surface, at most 96 m down. The Game may name its `lava`, a fluid that fills the
deepest caverns, and up to 16 `cave-biomes`, regions of the underground chosen
in three dimensions, each with its own `rock`, cavern `floor`, `features` on dry
floors and `sky-fog`. A Biome may list up to 16 `surface-features`, solid
Materials or [Plants](#plants) the Engine stands one Voxel above the Biome's
surface Material at a mean spacing in metres (see [Data Mods](data-mods.md)); a
Mod still places no terrain itself. A Biome may also list `structures` and
`settlements`, the [Templates](#templates) and [Settlements](#settlements)
worldgen stands in it. Any Mod may declare Sounds and Sound sets and give a
Sound set to any Mod's Material, and Soundscapes of ambient beds and music; only
the Game's Soundscapes take a `context`, which the Engine plays as the Player's
Eye goes underground or underwater (see [Data Mods](data-mods.md)).

| Table                       | Fields                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `[[materials]]`             | `name`, `emission` (0 to 15), `dimming` (0 to 15), `occlusion`, `render`, `solid`, optional `remove-time-s`, optional `contact-damage-per-s` (0 to 1000 health per second while a Player's body overlaps it), optional `tags` (at most 8 labels, `[a-z][a-z0-9-]{0,31}`; a Plant's `soil`, `stands-in` and water `fluid` may name `#tag`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `[[shapes]]`                | `name`, `boxes`, optional `collision`, optional `connect`, optional `form` (`box`, `cross` or `plane`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| box in `boxes`              | `min`, `max`: three integers, sixteenths of a Voxel                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| box in `collision`          | `min`, `max`: three signed integers, sixteenths of a Voxel                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `[shapes.connect]`          | `group`, `connects-to`, `to-sealed`, `arms` (each: `face`, `boxes`, optional `collision`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `[[appearance]]`            | `material` (a local name), `color` (four RGBA bytes), optional `texture` (a PNG path), optional `variation` (`fixed`, `turned` or `turned-and-patched`: whether each Voxel turns or mirrors the texture, and whether patches about 5 m across shift its colour; omitted is `fixed`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `[world]` (Game)            | optional `sea-level-m`, `relief-m`, `feature-size-m`, `soil-depth-m`, `octaves`, optional `lava` (a fluid by full name), and the tables below                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `[world.ground]`            | `rock`, `soil`, `surface`: full `<mod id>:<name>` Material names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `[world.caves]`             | `feature-size-m`, `radius-m`, `floor-m`, `octaves`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `[world.smoothing]`         | `min-rise-m`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `[[world.cave-biomes]]`     | `name`, `temperature`, `humidity` (0 to 1), optional `altitude` (`[min, max]` metres above Sea level), optional `rock` and `floor` (solids by full name), optional `sky-fog`, optional `features` (at most 8, each `material`: a solid by full name, optional `shape`, `spacing-m`: 1 to 64); at most 16, from the Game                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `[world.water]`             | `level-m`, `material` (a fluid, by full name); `level-m` is not read, as the water stands at `sea-level-m`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `[world.climate]` (Game)    | `temperature-size-m`, `humidity-size-m` (metres; omitted keeps 512 m)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `[world.movement]`          | `allow-flight`, `allow-physics`, `start-in-flight`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `[world.map]` (Game)        | `enabled` (`false` forbids the Map and the Compass in the World; omitted permits them)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `[world.sky]` (Game)        | `day-length-s` (at least 60), `start-hour` (0 up to 24), `day-share` (above 0, below 1), `tilt-deg` (0 to 60, toward the south), optional `day-curve` (1 to 16 keys; see [Data Mods](data-mods.md)), optional `clouds` (`altitude-m`: above the terrain band, `thickness-m`: 1 to 200, `cover`: 0 to 1, `wind-direction-deg`, `wind-speed-m-per-s`: 0 to 50); omitted holds the World at noon                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `[world.character]`         | `pixel-m`, `skin` (a 64 by 64 PNG path), `idle`, `walk`, `sprint`, `airborne`, `crouch` (each `swing-deg`, `rate-hz`, `arm-raise-deg`), `bones`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `[[world.character.bones]]` | `name`, optional `parent`, `role`, `pivot`, `cubes` (each `origin`, `size`, `uv`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `[view]` (Game)             | `view-distance-m`, `lod-distance-m` (the ceiling), `low-lod-distance-m` (optional: the Low preset's reach)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `[[fluids]]`                | `material`, `reach-m`, `sideways-m-per-s`, `downward-m-per-s`, `recession-m-per-s`, `reset-drop-m`, `waterlogs`, `extinguishes`, optional `heat` (`strength`, `reach-m`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `[[reactions]]`             | `first`, `second` (each `fluid`, `state`), `consumed`, optional `product`, optional `receiving`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `[[combustion]]`            | `fuel`, `ignition-threshold`, `ignition-delay-s`, `emission`, optional `heat`, `end`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `[[tools]]` (Game)          | `name`, `kind`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `[[items]]`                 | `name`, `size-m` (0.05 to 0.5 m), `color`, optional `material` (solid; at most one item per Material; removing a Voxel of it drops the item), optional `max-stack` (1 to 64; omitted is 64), optional `food` (hunger restored, above 0 and at most 1000), optional `texture` (a 16 by 16 PNG path: a flat sprite)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `[[plants]]`                | `name`, `kind`, `seed` (an Item), optional `shape`, `[plants.needs]`, then by kind: `stages`, optional `harvest-to` and `fruit-vine` (`crop`); `sapling`, `trunk`, `canopy`, `trunk-height-m`, `canopy-radius-m`, `mean-seconds`, the yields, optional `fruit`, `vines`, `branching` and `crowding-m` (`tree`); `material`, `max-height-m`, `direction`, `mean-seconds`, optional `stands-in` (`column`); `material`, `reach-m`, `mean-seconds`, `density`, optional `min-neighbours` (`spreader`); `material`, `mean-seconds`, `length-m` (`climber`); 128 per Mod, 512 in a World                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `[[biomes]]`                | `name`, `temperature`, `humidity` (both 0 to 1), optional `ground` (`rock`, `soil`, `surface`: solid Materials by full name), optional `sky-fog` (`sky`, `fog`: sRGB `[r, g, b]`), optional `cave-scale` (0 to 4), optional `cloud-cover` (0 to 4), optional `surface-features` (at most 16, each `material`: a solid by full name, or `plant`: a Plant by full name, optional `shape`, `spacing-m`: 1 to 1024, optional `clump`: `{ scale-m, strength }`, optional `field`: `{ group, scale-m, strays }`), optional `crop-fields` (at most 2, each `crops`: `[plant, weight]` pairs, `plot-m`: `[low, high]`, `spacing-m`, optional `path`, `ditch`, `near-water-m`), optional `structures` (at most 4, each `template`: by full name, `spacing-m`: twice the Template's longer side to 4096, `chance`: above 0, at most 1), optional `settlements` (at most 2, each `settlement`: by full name, `spacing-m`: twice its radius and margin to 4096, optional `chance`: above 0, at most 1, 1 when omitted), optional `loot` (a [loot table](#loot)), optional `selectors` (`altitude`, `coast`, `erosion`, `river`: each `[min, max]`; `relief`: 0 to 4), optional `strata` (at most 8, each `material`: a solid by full name, `depth-min-m`, `depth-max-m`: at most 96); at most 16 per Mod and in a World |
| `[[templates]]`             | `name`, `cell-m` (metres), `palette` (one character to `"keep"`, `"air"`, `"ground"` or `{ material, shape, orientation }`), `layers` (bottom up, each rows along +Z of characters along +X), `ground-layer`, optional `foundation` (a solid by full name), optional `level-tolerance-m` (0 to 16; omitted is 1), optional `spans` (a bridge; omitted is false); at most 64 m a side and 65,536 Template cells, 64 in a World                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `[[settlements]]`           | `name`, `radius-m` or `radius-max-m` (above 0, at most 160), optional `radius-min-m` (omitted is `radius-m`), optional `margin-m` (0 to 64; omitted is 16), optional `centre` (a Template by full name), `plots` or `plots-max` (at most 96), optional `plots-min` (omitted is `plots`), `templates` (1 to 96, each `template`: by full name, optional `weight`: above 0, omitted is 1, `min`: omitted is 0, `max`: omitted is 96), optional `minimum` (omitted is 0), `path` (a solid by full name), optional `path-shape`, optional `loot` (a [loot table](#loot), also on a `templates` row), optional `outskirts` (one table, or a list of up to 4 groups laid in order, each `templates` as above, each no wider than `margin-m`, `count-min`, `count-max`: at most 96, optional `chance`: above 0, at most 1; a copy that cannot stand `count-min` lays none of the group); 16 in a World                                                                                                                                                                                                                                                                                                                                                                                                             |
| `[[marker-kinds]]`          | `name`, `icon` (`home`, `camp`, `mine`, `farm`, `landmark`, `death`, `flag`, `pin` or `star`), `color` (four RGBA bytes); 16 per Mod                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `death-marker-kind` (Game)  | the full `<mod id>:<name>` of the Marker kind a death leaves; a top-level key, before any table                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `[survival]` (Game)         | each optional, omitted keeps the engine's default: `max-health` (20), `max-hunger` (20), `max-air-s` (10), `safe-fall-m` (3), `fall-damage-per-m` (1), `drowning-damage-per-s` (2), `hunger-per-min` (1), `sprinting-hunger-per-min` (2), `regeneration-hunger` (18), `regeneration-per-s` (0.25), `starvation-damage-per-s` (0.25), `keep-inventory` (false)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `[[ores]]`                  | `material` (solid), `min-depth-m`, `max-depth-m`, `vein-size-m`, `rarity`, optional `biomes`; 16 per Mod, 32 in a World                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `[[sounds]]`                | `name`, `category`, `recordings` (at least one, each `file`: an Ogg Vorbis path, `weight`), `volume` and `pitch` (each `min`, `max`), optional `distance` (`reference-m`, `max-m`: required for `effects`, refused for the others)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `[[sound-sets]]`            | `name`, optional `break`, `place`, `hit`, `step`, `land`: each an `effects` Sound by full name                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `[[material-sounds]]`       | `material`, `sound-set`: both by full name; at most one Sound set per Material in a World                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `[[recipes]]`               | `inputs` (1 to 9 different Items, each `item`: by full name, `count`: at least 1), `output` (`item`, `count`: 1 to the Item's `max-stack`), `station` (`"hand"` or a Station's tag); 256 per Mod, 1024 in a World                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `[[voxel-uses]]`            | `kind`, `material` (solid, by full name), then by kind: `open` for a `door` (solid, by full name) and optional `width-m` (0.5 to 4; omitted is 1) and `height-m` (1 to 4; omitted is 2), optional `slots` for a `container` (1 to 54; omitted is 27), `tag` for a `station`; optional `shape` for any kind (a Shape by full name); 64 per Mod                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `[[cause-sounds]]` (Game)   | `cause`, `sound` (an `effects` Sound by full name); each Cause at most once                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `[[soundscapes]]`           | `name`, optional `context` (Game only, at most one each), `beds` (at most 8, each `sound`: an `ambient` Sound by full name, `volume`: 0 to 1), `music` (at most 8 `music` Sounds by full name), optional `silence-s` (`min`, `max`: 0 to 3600 s, default 60 to 180); 16 per Mod                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |

Variants: `occlusion` is `opaque`, `same-material` or `never`; `render` is
`opaque`, `blend` or `cutout`; a Sound `category` is `effects`, `ambient` or
`music`; a Soundscape `context` is `default`, `underground` or `underwater`; a
bone `role` is `body`, `head`, `arm-left`, `arm-right`, `leg-left`, `leg-right`
or `fixed`; a reactant `state` is `any`, `source` or `flowing`; `consumed` and
`receiving` are `first` or `second`; a tool `kind` is `measure` or `place`; a
voxel-use `kind` is `door`, `container` or `station`; a Plant `kind` is `crop`,
`tree`, `column`, `spreader` or `climber`; a Shape `form` is `box`, `cross` or
`plane`; a column `direction` is `up` or `down`. A combustion `end` is
`"permanent"` or `{ after = { seconds = 8, residue = "mod:ash" } }`, and an
omitted `residue` removes the solid.

Materials, Shapes, Biomes, Plants, Marker kinds, Sounds, Sound sets,
Soundscapes, Templates and Settlements are declared by local name. Ground,
water, fluid, reaction, combustion, ore and surface feature names, the Biomes an
ore lists, the names in Sound sets, `material-sounds` and Soundscapes, the
Items, Materials, Shapes and `#tag`s in `recipes`, `voxel-uses` and `plants`,
and every name in `templates`, `settlements` and a Biome's `structures` and
`settlements`, are full `<mod id>:<name>` names and resolve after every Mod has
declared, whether the Material came from data or from a component. A Recipe may
name a Material's implicit Item, which has the Material's full name.

### A Declaration using every table

This Game is checked by the repository's tests: it is extracted from this page,
given the two PNGs and the recordings it names, and validated.

```toml
[[materials]]
name = "stone"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true
remove-time-s = 0.5

[[materials]]
name = "water"
emission = 0
dimming = 2
occlusion = "same-material"
render = "blend"
solid = false

[[materials]]
name = "lava"
emission = 15
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = false

[[materials]]
name = "wood"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "ash"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "iron"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "bench"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "crate"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "hatch"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "hatch_open"
emission = 0
dimming = 0
occlusion = "never"
render = "blend"
solid = true

[[shapes]]
name = "post"
boxes = [{ min = [6, 0, 6], max = [10, 16, 10] }]
collision = [{ min = [6, 0, 6], max = [10, 24, 10] }]

[shapes.connect]
group = 1
connects-to = 1
to-sealed = true
arms = [
  { face = 0, boxes = [{ min = [0, 7, 7], max = [6, 13, 9] }], collision = [{ min = [0, 0, 7], max = [6, 24, 9] }] },
  { face = 1, boxes = [{ min = [10, 7, 7], max = [16, 13, 9] }] },
]

[[appearance]]
material = "stone"
color = [120, 120, 128, 255]
texture = "art/stone.png"

[[appearance]]
material = "water"
color = [26, 82, 158, 166]

[[appearance]]
material = "lava"
color = [230, 72, 16, 255]

[[appearance]]
material = "wood"
color = [150, 104, 60, 255]

[[appearance]]
material = "ash"
color = [70, 66, 62, 255]

[world]
sea-level-m = 24.0
relief-m = 16.0

[world.ground]
rock = "showcase:stone"
soil = "showcase:stone"
surface = "showcase:stone"

[world.caves]
feature-size-m = 40.0
radius-m = 3.0
floor-m = 6.0
octaves = 2

[world.smoothing]
min-rise-m = 1.0

[world.water]
level-m = 22.0
material = "showcase:water"

[world.climate]
temperature-size-m = 400.0
humidity-size-m = 600.0

[world.movement]
allow-flight = true
allow-physics = true
start-in-flight = false

[world.character]
pixel-m = 0.05625
skin = "skin.png"
idle = { swing-deg = 0.0, rate-hz = 0.0, arm-raise-deg = 0.0 }
walk = { swing-deg = 40.0, rate-hz = 1.6, arm-raise-deg = 0.0 }
sprint = { swing-deg = 55.0, rate-hz = 2.2, arm-raise-deg = 0.0 }
airborne = { swing-deg = 10.0, rate-hz = 0.0, arm-raise-deg = 20.0 }
crouch = { swing-deg = 20.0, rate-hz = 1.0, arm-raise-deg = 0.0 }

[[world.character.bones]]
name = "body"
role = "body"
pivot = [0.0, 12.0, 0.0]
cubes = [{ origin = [-4.0, 12.0, -2.0], size = [8, 12, 4], uv = [16, 16] }]

[[world.character.bones]]
name = "head"
parent = "body"
role = "head"
pivot = [0.0, 24.0, 0.0]
cubes = [{ origin = [-4.0, 24.0, -4.0], size = [8, 8, 8], uv = [0, 0] }]

[view]
view-distance-m = 160.0
lod-distance-m = 3000.0

[[fluids]]
material = "showcase:water"
reach-m = 7.0
sideways-m-per-s = 4.0
downward-m-per-s = 8.0
recession-m-per-s = 4.0
reset-drop-m = 0.5
waterlogs = true
extinguishes = true

[[fluids]]
material = "showcase:lava"
reach-m = 3.0
sideways-m-per-s = 0.75
downward-m-per-s = 1.5
recession-m-per-s = 0.75
reset-drop-m = 0.5
waterlogs = false
extinguishes = false
heat = { strength = 12, reach-m = 2.0 }

[[reactions]]
first = { fluid = "showcase:lava", state = "any" }
second = { fluid = "showcase:water", state = "any" }
consumed = "first"
product = "showcase:stone"

[[combustion]]
fuel = "showcase:wood"
ignition-threshold = 6
ignition-delay-s = 2.0
emission = 12
heat = { strength = 12, reach-m = 2.0 }
end = { after = { seconds = 8.0, residue = "showcase:ash" } }

[[tools]]
name = "measure"
kind = "measure"

[[items]]
name = "pebble"
size-m = 0.1
color = [150, 150, 150, 255]

[[biomes]]
name = "dry"
temperature = 0.8
humidity = 0.15

[[biomes]]
name = "cold"
temperature = 0.1
humidity = 0.4
ground = { rock = "showcase:stone", soil = "showcase:ash", surface = "showcase:wood" }
sky-fog = { sky = [150, 164, 184], fog = [176, 190, 208] }
surface-features = [{ material = "showcase:iron", spacing-m = 32.0 }]
structures = [{ template = "showcase:hut", spacing-m = 64.0, chance = 0.5 }]
settlements = [{ settlement = "showcase:hamlet", spacing-m = 96.0 }]

[[ores]]
material = "showcase:iron"
min-depth-m = 4.0
max-depth-m = 40.0
vein-size-m = 3.0
rarity = 0.05
biomes = ["showcase:cold"]

[[templates]]
name = "hut"
cell-m = 1.0
ground-layer = 1
foundation = "showcase:stone"
layers = [
  ["gggg", "g..g", "g..g", "gggg"],
  ["wwww", "w.ww", "wwww", "wwww"],
  ["ssss", "s..s", "s..s", "s..s"],
  ["ssss", "s..s", "s..s", "s..s"],
  ["_rr_", "rrrr", "rrrr", "_rr_"],
]

[templates.palette]
"s" = { material = "showcase:stone" }
"w" = { material = "showcase:wood" }
"r" = { material = "showcase:wood", shape = "core:slab" }
"g" = "ground"
"." = "air"
"_" = "keep"

[[settlements]]
name = "hamlet"
radius-m = 16.0
plots = 4
minimum = 2
path = "showcase:stone"
templates = [{ template = "showcase:hut", weight = 1.0, min = 1, max = 4 }]

[[sounds]]
name = "crack"
category = "effects"
recordings = [{ file = "sounds/crack.ogg", weight = 1 }]
volume = { min = 0.8, max = 1.0 }
pitch = { min = 0.9, max = 1.1 }
distance = { reference-m = 2.0, max-m = 24.0 }

[[sound-sets]]
name = "rock"
break = "showcase:crack"

[[cause-sounds]]
cause = "craft"
sound = "showcase:crack"

[[material-sounds]]
material = "showcase:stone"
sound-set = "showcase:rock"

[[sounds]]
name = "wind"
category = "ambient"
recordings = [{ file = "sounds/wind.ogg", weight = 1 }]
volume = { min = 1.0, max = 1.0 }
pitch = { min = 1.0, max = 1.0 }

[[sounds]]
name = "theme"
category = "music"
recordings = [{ file = "sounds/theme.ogg", weight = 1 }]
volume = { min = 0.8, max = 0.8 }
pitch = { min = 1.0, max = 1.0 }

[[soundscapes]]
name = "open"
context = "default"
beds = [{ sound = "showcase:wind", volume = 0.6 }]
music = ["showcase:theme"]
silence-s = { min = 60.0, max = 180.0 }

[[soundscapes]]
name = "cave"
context = "underground"
beds = [{ sound = "showcase:wind", volume = 0.2 }]

[[voxel-uses]]
kind = "station"
material = "showcase:bench"
tag = "bench"

[[voxel-uses]]
kind = "container"
material = "showcase:crate"
slots = 9

[[voxel-uses]]
kind = "door"
material = "showcase:hatch"
open = "showcase:hatch_open"

[[recipes]]
inputs = [{ item = "showcase:wood", count = 4 }]
output = { item = "showcase:bench", count = 1 }
station = "hand"

[[recipes]]
inputs = [{ item = "showcase:wood", count = 6 }, { item = "showcase:iron", count = 1 }]
output = { item = "showcase:crate", count = 1 }
station = "bench"
```

Its `mod.toml` is the data-only starter's with `id = "showcase"`.

## Recipes and Usable Voxels

A Recipe is data: the Items a Craft spends, the one Item it makes, and where it
is crafted. `station` is required. `"hand"` makes a hand Recipe; any other value
is a Station's tag, so a forgotten field is a load error rather than an
accidental hand Recipe.

|                  | Hand Recipe                        | Station Recipe                                   |
| ---------------- | ---------------------------------- | ------------------------------------------------ |
| Declared as      | `station = "hand"`                 | `station = "workbench"`                          |
| Block needed     | None: craftable anywhere, any time | A Station of that tag placed in the World        |
| Opened by        | I, then the Crafting tab           | Aim at the Station and press Use (G)             |
| Where you can be | Anywhere                           | Within reach of it; walking away closes the pane |
| Lists            | Hand Recipes only                  | That Station's Recipes only                      |

Both take their inputs from the Inventory and give to it; what does not fit
drops at the Player. A Craft is instant, makes at most one output Stack, and is
not an Edit, so undo leaves it alone.

A `voxel-uses` row makes a solid Material usable. The Server runs each kind
natively; no Mod is called when a Voxel is used
([ADR-0026](adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md)).

- `door`: a size in metres, `width-m` by `height-m`, 1 by 2 when omitted, so it
  is as large at every Voxel size: one Voxel thick and as many Voxels wide and
  tall as that size needs. One Item places the whole Door across the Player's
  facing, as one undo step, refused if any of it is blocked. Use swaps
  `material` and `open`, two solid Materials, over all of it. With a fixed
  `shape` (below) each Voxel is that Shape drawn closed against its own +Z:
  placing turns it to face the Player, hinged on the half of the cell they aimed
  at, and Use turns it a quarter about that hinge edge. An open Voxel whose
  neighbour on its hinge side is the same open Voxel is folded into its leaf: it
  draws nothing and aim passes it, so a leaf wider than one Voxel opens as one
  panel against its jamb and a pair opens from the middle. Bodies collide with
  what is drawn, except that an open Door with no fixed Shape, which keeps the
  Player's Shape and Orientation, or one wider than a Voxel lets them through.
  Aim, Use and Remove stop on what is drawn. Removing any Voxel of a Door
  removes it all and drops one of the closed one's Item; the open Material gets
  no implicit Item and cannot be placed. Doors side by side stay two Doors.
  Toggling never enters the undo journal.
- `container`: the Voxel holds `slots` Stacks of its own, shown beside the
  Inventory. A Player cannot break one that holds Stacks; anything else that
  removes it spills them as Dropped items.
- `station`: Use opens the Recipes whose `station` is its `tag`. Materials may
  share a tag, so another Mod's bench opens the same Recipes. `hand` is
  reserved.

Any row may set `shape`, a Shape by full name (`core:slab`, or a Mod's own),
which a Player's Place of its Material always uses whatever Shape they chose:
`timber`'s 2 by 3 m Door and its 1 m Gate are a thin `timber:leaf`, and
`workshop`'s Workbench a `workshop:table`. A Station or Container keeps the
Player's Orientation; a Door is turned as above. A Shape any row fixes is that
row's alone: the Player is not offered it to place other Materials in.

#### Choosing a Door's hinge

A Door's panel opens onto its hinge side. A Player picks it by aim: placing a
Door hinges it on the half of the cell the crosshair is on, left or right of the
middle. A Template picks it by each Door cell's orientation byte: upright (`up`
is +Y), the panel's own +Z face is its front and its hinge is the side its own
+X turns to; upside down (`up` is -Y) the front stays and the hinge swaps sides,
the only way to mirror it. So with the panel on +Z, orientation 0 hinges on +X
and 6 on -X. The Structure's turn turns both. Hinge a single Door away from the
nearer corner of its wall, and the two leaves of a double Door on their outer
edges, so they open from the middle; the demo Games' mockup says it per Door
cell with `hinge: "left"` or `"right"`, seen from outside.

A Material is in at most one row, as its `material` or as a Door's open one. Any
Mod may add Recipes for another Mod's Items and Stations; names resolve after
every Mod has declared, and an unknown Item or tag is a load error naming the
Recipe. Both starters declare one of each kind and one Recipe of each sort.

## Plants

A Plant is data the Server grows
([ADR-0027](adr/0027-plants-are-declared-kinds-the-server-grows.md)). A
`[[plants]]` row has a `name` (local to the Mod), a closed `kind`, a `seed`, an
optional `shape` and a `[plants.needs]` table. No Mod is called when a Plant
grows, and the Engine ships none. At most 128 per Mod and 512 in a World. Every
size is in metres and every rate in seconds, so a Plant behaves the same at
every Voxel size.

- **A Plant's state is its Material.** A crop's Stage is a Material, so a crop
  of four Stages declares four Materials. A Material is at most one Stage of one
  Plant, once. Only the first Stage gets an Item, the `seed`, which places it;
  every later Stage, a tree's trunk and canopy and fruit have no implicit Item
  and cannot be placed.
- **`seed`** is an Item whose `material` is the first Stage, a tree's sapling or
  the one Material of any other kind.
- **`shape`** is the Shape of the Plant's single-Voxel Stages, by full name.
  Placing the seed, growth and generation all use it whatever the Player chose,
  and a `plane` is turned to the face the Player aimed at. Omitted, the Player's
  choice and cubes.
- **`[plants.needs]`**: `soil`, a list of solid Materials or `#tag`s (at least
  one) the Plant stands on, or for a climber holds to; optional `light-min` and
  `light-max` (0 to 15, at the Plant's Voxel); optional `temperature` and
  `humidity`, each `[low, high]` from 0 to 1 against the Biome's Climate; and
  optional `water = { fluid, reach-m }`. A Plant that fails a need holds its
  Stage and never dies of it. Only `soil` is a placement rule: it refuses a Seed
  ("Wheat needs soil.") and breaks a Plant whose soil goes. Needs are read when
  a Sample passes, so a Plant in the dark waits.
- **`yields`** on a Stage (or a column, spreader or climber) is a list of
  `{ item, count = [min, max] }` a Voxel drops when removed or picked. Counts
  are a hash of seed and position, the same in Creative. A Stage with none drops
  its Seed.
- **`shape`** on a crop or fruit Stage is that Stage's own Shape, by full name;
  omitted, the Plant's (or the fruit's). A Voxel takes it when it advances or a
  ripe `harvest-to` crop resets, keeping its Orientation, so the berry bush is a
  stem, a mound and a mound with berries, and unripe fruit is a smaller Shape
  than ripe. A seed places the first Stage's Shape. A Stage's tile paints each
  of its boxes by position, as for any small model.
- **`mean-seconds`** is the average time a Stage takes to reach the next, at
  least 16, the Sample interval. A Stage takes about that long on average, not
  exactly.

### Tags

A Material may carry up to 8 `tags` (`[a-z][a-z0-9-]{0,31}`). `soil`, a column's
`stands-in` and a water `fluid` may name `#<tag>` for every Material of every
loaded Mod that carries it, solid ones for soil and fluids for water. A tag that
matches none is a load error naming the Plant. `strata` tags `turf`, `soil` and
`clay` as `soil`, `sand` as `sand`, `stone` and `basalt` as `rock`, and `waters`
tags its water `water`, so a shared Mod can say `#soil` and grow in any Game
whose ground carries it.

### The five kinds

A `crop` is one Voxel through 2 to 8 `stages`, each a Material with
`mean-seconds` to reach the next; the last is ripe. Breaking it at any Stage
drops that Stage's yield. `harvest-to`, an earlier Stage by index, makes a
Remove (breaking) of the ripe Stage give its yield and reset to that Stage, so
the crop regrows; to remove it whole, break it at the reset Stage or earlier. A
crop without `harvest-to` is uprooted. Use harvests nothing. A tree's fruit is
broken: a ripe one drops its yield, an unripe one nothing, and canopy is left in
its place. A `spreader` such as a mushroom is picked and regrows only by
spreading from a neighbour. A crop may also be a vine crop, below. From `crops`:

```toml
[[plants]]
name = "wheat"
kind = "crop"
seed = "crops:wheat_seeds"
shape = "flora:cross"
stages = [
    { material = "crops:wheat_sprout", mean-seconds = 45.0, yields = [{ item = "crops:wheat_seeds", count = [1, 1] }] },
    { material = "crops:wheat_young", mean-seconds = 45.0, yields = [{ item = "crops:wheat_seeds", count = [1, 1] }] },
    { material = "crops:wheat_green", mean-seconds = 45.0, yields = [{ item = "crops:wheat_seeds", count = [1, 1] }] },
    { material = "crops:wheat_ripe", yields = [{ item = "crops:wheat", count = [1, 2] }, { item = "crops:wheat_seeds", count = [1, 2] }] },
]

[plants.needs]
soil = ["#soil"]
light-min = 9
```

A `tree` is a sapling that grows a parametric tree as one group: a trunk
`trunk-height-m` tall (a `[low, high]` up to 32, chosen per tree) under an
ellipsoid canopy of `canopy-radius-m` (up to 8), after `mean-seconds` on
average. The group lands whole or the sapling Samples again. `sapling` is its
Material, `trunk` and `canopy` its **grown Materials**, made only by growth and
generation and named by no Item. Cutting a trunk fells what hung on it, top-down
over a few moments, and each Voxel drops `trunk-yield` or `canopy-yield`;
`sapling-yield` (at least 1) is given once for the whole tree. From `flora`,
trimmed of its extra yields:

```toml
[[plants]]
name = "oak"
kind = "tree"
seed = "flora:oak_sapling"
shape = "flora:cross"
sapling = "flora:oak_sapling"
trunk = "flora:oak_log"
canopy = "flora:oak_leaves"
trunk-height-m = [4.0, 6.0]
canopy-radius-m = 2.5
mean-seconds = 90.0
sapling-yield = [1, 2]

[plants.needs]
soil = ["#soil"]
light-min = 8
temperature = [0.2, 0.9]

[[plants.sapling-yields]]
item = "flora:oak_sapling"
count = [1, 1]

[[plants.trunk-yield]]
item = "flora:log"
count = [1, 1]

[[plants.canopy-yield]]
item = "flora:leaves"
count = [0, 1]
```

A tree may bear `fruit`: 2 to 4 grown `stages` (the last ripe), a `density` of
canopy Voxels from 0 to 0.3 that carry it, `regrow-seconds` (at least 16, 120 by
default) and a `shape` for every fruit Voxel, so fruit can be a small model of
boxes. A box face samples its Material's tile at the face's own place in the
Voxel (the side tile by horizontal position and height, the top and bottom tiles
by x and z), so one tile can paint a model's body, stem and leaf each in its own
colour; a berry bush and a mushroom are such box Shapes with empty `collision`.
Breaking a ripe one drops its yield and leaves canopy; while the tree stands it
grows a new unripe fruit on its canopy about every `regrow-seconds`, up to
`density` of its canopy, when its light and Climate hold, and a felled tree
grows none. It may also hang `vines = { plant, density, length-m }` from its
outer canopy, only when a World is generated, never when a sapling grows. The
vine is a `climber` with a `plane` Shape whose `soil` names the tree's trunk or
canopy Material. From `flora`'s apple and jungle trees:

```toml
fruit = { stages = [{ material = "flora:apple_unripe", shape = "flora:apple_unripe", mean-seconds = 40.0 }, { material = "flora:apple_ripe", yields = [{ item = "flora:apple", count = [1, 2] }] }], density = 0.12, regrow-seconds = 120.0, shape = "flora:apple" }
vines = { plant = "flora:vine", density = 0.12, length-m = 4.0 }
```

A tree may **branch**. Without `branching` it is a straight trunk under one
ellipsoid; with it the trunk can lean or fork and carry limbs, each ending in a
lobe of foliage. Everything is in metres or fractions of the trunk height, so
the tree is the same at every Voxel size:

| Field           | Meaning                                                                      |
| --------------- | ---------------------------------------------------------------------------- |
| `limbs`         | `[min, max]` limbs (at most 16), or tiers (at most 8) with `tiers = true`    |
| `start`         | fraction of the trunk height where limbs begin, 0 to 0.8                     |
| `reach-m`       | horizontal length of the longest limb; a fork's leaders spread half of it    |
| `rise`          | -1 drooping to 1 ascending; a tip stands `rise * reach-m / 2` above its root |
| `fork`          | optional height fraction (0.1 to 0.8) where the trunk splits into 2 or 3     |
| `lean-m`        | how far the trunk's top stands from its foot, up to the lowest trunk height  |
| `tiers`         | optional: limbs in rings of four that shorten towards the top, a cone        |
| `lobe-radius-m` | radius of the foliage lobe at each limb and leader tip, above 0              |

Limbs and leaders are the tree's `trunk` Material, one Voxel thick, so a cut
trunk fells them with everything above it and a cut limb drops its end and what
hung on it while the trunk stands. The crown keeps `canopy-radius-m`; the lean
plus the larger of the crown and the fork's spread, limb and lobe together must
stay within 8 m, and a fork's spread (half of `reach-m`) within three quarters
of the crown and lobe radii so the tree is one piece. From `flora`'s oak and
pine:

```toml
branching = { limbs = [3, 4], start = 0.25, reach-m = 3.5, rise = 0.3, fork = 0.3, lean-m = 0.0, lobe-radius-m = 1.5 }
branching = { limbs = [3, 5], start = 0.25, reach-m = 3.0, rise = -0.5, lean-m = 0.0, tiers = true, lobe-radius-m = 0.9 }
```

When a World is generated, a tree does not stand within `crowding-m` of a tree
whose draw beats it, so two crowns meet without closing into one roof. Absent,
it is 1.5 times `canopy-radius-m`; a species that stands alone in a dense Biome
declares less, from above 0 to 12, so its trunks can stand closer than its crown
is wide. From `flora`'s jungle tree:

```toml
crowding-m = 2.0
```

`canopy` takes one Material or a list of up to four, alike but for colour: each
tree grows one, drawn by the World seed and where it stands, so neighbouring
crowns of one species read as separate trees. Each is a grown Material. A fruit
picked from a tree leaves the first.

`storeys` splits a row into up to four height bands. Each has a `weight`, its
share of the row's trees, a `trunk-height-m` and an optional `canopy-radius-m`
(the row's when absent); each tree belongs to one, drawn like its canopy, so a
forest has trees above and below its canopy. A storey's `branching` is the
row's, its reach, lean and lobes scaled by its crown over the row's
`canopy-radius-m`. Where two trees' crowns meet the taller storey's Voxel
stands. A row without `storeys` is one band, `trunk-height-m` under
`canopy-radius-m`. From `flora`'s swamp tree:

```toml
canopy = ["flora:swamp_leaves", "flora:swamp_leaves_dark", "flora:swamp_leaves_light"]
storeys = [
  { weight = 0.7, trunk-height-m = [12.0, 15.0] },
  { weight = 0.3, trunk-height-m = [6.0, 9.0], canopy-radius-m = 2.5 },
]
```

A `column` adds one Voxel at the end of its run, `direction` up or down, to
`max-height-m` (up to 32), one every `mean-seconds`. `stands-in` names the fluid
it grows through and stands waterlogged in (kelp); omitted, it grows in Air.
From `aquatic`:

```toml
[[plants]]
name = "kelp"
kind = "column"
seed = "aquatic:kelp"
shape = "flora:cross"
material = "aquatic:kelp"
max-height-m = 5.0
direction = "up"
mean-seconds = 90.0
stands-in = "#water"

[plants.needs]
soil = ["#sand", "#soil", "#rock"]
```

A `spreader` copies itself onto a hash-picked Voxel within `reach-m` (1 to 8)
that meets its needs, one attempt per `mean-seconds`, while at most `density` of
its Voxels stand within reach, so a field does not fill solid. The optional
`min-neighbours` (default 0, below `density`) adds a floor: a target is seeded
only where at least that many of its Voxels stand within `reach-m` of it,
counting the seeding one, so a patch fills its holes and does not advance. Both
are counts of Voxels, so tune them together at the Voxel size the Game expects:
`min-neighbours` just under what a worldgen patch holds within reach keeps it
where it is, and the closer to `density` the tighter. It slows creep rather than
forbidding it, since a cluster at the floor's own density can still crawl. A
Voxel with none of its kind within its own reach is exempt, so a single planted
seed still grows one neighbour; a gardener plants a handful to start a patch.
From `flora`:

```toml
[[plants]]
name = "tuft"
kind = "spreader"
seed = "flora:tuft"
shape = "flora:cross"
material = "flora:tuft"
reach-m = 2.0
mean-seconds = 900.0
density = 5
min-neighbours = 4

[plants.needs]
soil = ["#soil"]
light-min = 9
```

A vine crop is a `crop` with `fruit-vine`, such as the watermelon and pumpkin of
`crops`: its last (ripe) Stage bears fruit on the ground round it, as a tree
bears fruit on its canopy. `fruit-vine` holds 2 to 4 fruit `stages` (the last
ripe, each with an optional `shape`), `regrow-seconds` (at least 16) between new
fruit on a vine with room, and optional `shape`, `reach-m` (1.5 m, up to 8),
`gap-m` (0.5 m, up to `reach-m`) and `max-fruit` (5, up to 16). On a passing
Sample the ripe vine picks a free cell within reach that has the Plant's `soil`
below and its light and Climate, and bears an unripe fruit there if fewer than
`max-fruit` of its fruit stand in reach and none within the gap, which is
`gap-m` in Voxels rounded up and at least one, so fruit never touches. Fruit
never seeds, so a cluster does not creep; breaking one, which drops that Stage's
yield, lets the vine bear another. Fruit Materials are grown, so they have no
Item and cannot be placed, and fruit with no soil under it breaks. A vine crop
has no `harvest-to`. A Biome feature naming a vine crop stands the ripe vine
with its first fruit, a pure function of the seed and the vine's column in
metres. A fruit Shape with `collision` blocks bodies, as the melons and pumpkins
of `crops` do; one without, like a berry, is walked through. From `crops`:

```toml
[[plants]]
name = "watermelon"
kind = "crop"
seed = "crops:watermelon_seeds"
shape = "flora:cross"
stages = [
    { material = "crops:watermelon_sprout", mean-seconds = 60.0 },
    { material = "crops:watermelon_young", mean-seconds = 60.0 },
    { material = "crops:watermelon_vine" },
]
fruit-vine = { stages = [{ material = "crops:watermelon_unripe", shape = "crops:watermelon_unripe", mean-seconds = 60.0 }, { material = "crops:watermelon_ripe", yields = [{ item = "crops:watermelon_slice", count = [3, 5] }] }], regrow-seconds = 240.0, shape = "crops:watermelon" }

[plants.needs]
soil = ["#soil"]
light-min = 9
```

A `climber` adds a Voxel on an adjacent face of a solid, or hangs below an
overhang, until `length-m` (up to 32) Voxels are face-connected. Its Shape is a
`plane`, which takes the face it holds to. From `flora`, with its soil list
shortened:

```toml
[[plants]]
name = "vine"
kind = "climber"
seed = "flora:vine"
shape = "flora:plane"
material = "flora:vine"
mean-seconds = 90.0
length-m = 6.0

[plants.needs]
soil = ["#rock", "#soil", "flora:wood", "flora:leaves"]
```

### Flat Shapes and item textures

A Shape's `form` is `box` (the default), `cross` (two diagonal quads through the
Voxel's centre) or `plane` (one quad against a face). A flat form still declares
`boxes`, which decide selection and where fluid can stand, so a plant can be
aimed at and kelp is waterlogged where it grows. It is walked through unless the
Shape declares `collision`, seals no face, cannot `connect`, and costs nothing
against the Chunk's shaped-Voxel cap. Light passes by the Material's own
`dimming`, so a plant's Material is `render = "cutout"` and
`occlusion = "never"` with a `dimming` of 0 to 2. From `flora`:

```toml
[[shapes]]
name = "cross"
form = "cross"
boxes = [{ min = [3, 0, 3], max = [13, 14, 13] }]

[[shapes]]
name = "plane"
form = "plane"
boxes = [{ min = [1, 1, 14], max = [15, 15, 16] }]
```

An Item whose Material's Shape is flat is drawn as a flat sprite from that
Material's side texture in the hand, the hotbar, the Inventory and on the
ground. An Item with no placeable Material, such as wheat or berries, sets
`texture`, a 16 by 16 PNG: it is drawn flat from that sprite, and without one it
is a cube of `color`.

### Biomes place Plants

A Biome's `surface-features` take at most 16 entries, each `material` or `plant`
(exactly one), an optional `shape` that overrides a Plant's own, and
`spacing-m`:

```toml
surface-features = [{ plant = "flora:oak", spacing-m = 11.0 }, { plant = "flora:tuft", spacing-m = 6.0 }]
```

Each stands about once in every `spacing-m` by `spacing-m` metres of the Biome's
surface, so chances add up. If a Biome's sum passes 1 the earlier entries take
the column and the later ones never stand there, so list rare trees first and
ground cover last. A `crop` or `spreader` stands at a Stage drawn by hash, a
`column` on the ground or, with `stands-in`, in the lake, and a `tree` fully
grown, whole across Chunk borders. A `climber` and a column growing down are
refused.

Tune a Biome by density class: trees 3 to 5 m apart with a 1 to 2 m understory
for a jungle or swamp, 7 to 10 m for a temperate or cold forest, 20 to 40 m for
a savanna, prairie, desert or frost Biome, with ground cover finer than trees. A
`clump` gathers a feature into groves and clearings without changing its mean
spacing:

```toml
surface-features = [{ plant = "flora:oak", spacing-m = 9.0, clump = { scale-m = 48.0, strength = 0.8 } }]
```

`scale-m` (4 to 1024) is how far the density varies, a patch's spacing, and
`strength` (0 to 1) how hard: 0.6 gives groves at up to 1.6 times the density
with thin clearings, 0.95 patches over about a fifth of the ground at up to 5
times the density with bare ground between, and 1 patches at 6.6 times with
nothing between them. The field is a function of the seed and the position in
metres, so it is the same at every Voxel size.

Ground cover reads best as patches 4 to 10 m across with bare ground between
them: a `scale-m` of 12 to 16 and a `strength` of 0.9 to 0.95, at a mean spacing
that keeps it 10 to 15% of a grassy Biome and under 8% of a sparse one. The
clump places the patches at generation; a spreader's `density` caps the
neighbours within its `reach-m` and not how far its field creeps, so give ground
cover a `mean-seconds` of 15 minutes or more and it grows a patch a few Voxels
in an hour and takes many hours to thin out over the Biome.

### Fields

Two optional parts of a Biome make ground read as land that is used, both pure
functions of the seed and positions in metres, so they are whole across Chunk
borders and the same at every Voxel size.

A **wild field** groups features of a Biome so a patch is mostly one species, as
a meadow's flowers are. Give each member the same
`field = { group, scale-m, strays }`:

```toml
surface-features = [
    { plant = "flora:dandelion", spacing-m = 7.0, clump = { scale-m = 12.0, strength = 0.95 }, field = { group = "flowers", scale-m = 20.0, strays = 0.12 } },
    { plant = "flora:daisy", spacing-m = 6.5, clump = { scale-m = 12.0, strength = 0.95 }, field = { group = "flowers", scale-m = 20.0, strays = 0.12 } },
]
```

A jittered lattice `scale-m` apart (8 to 96 m) cuts the ground into regions, and
each region is given one member of the group by hash. A column that draws a
member stands it only in a region that holds it or, with chance `strays` (0 to
0.5), anyway. Every member of a group gives the same `scale-m` and `strays`, and
a group of one changes nothing. A member stands in only its share of the ground,
so its `spacing-m` is thinner than it reads: close it by the root of the share,
`sqrt((1 + strays * (members - 1)) / members)`, to keep the group's cover. A
column the group's draw leaves bare stays bare; the next feature never takes it.
A feature with no `field` generates as before.

A **planted field** is a `crop-fields` entry, at most 2 to a Biome:

```toml
crop-fields = [
    { crops = [["crops:wheat", 6.0], ["crops:carrot", 2.0], ["crops:potato", 2.0]], plot-m = [8.0, 16.0], spacing-m = 72.0, path = "strata:sand", ditch = "waters:water", near-water-m = 40.0 },
]
```

A lattice cell `spacing-m` on a side (32 to 256) draws whether it holds a field,
and a held cell tries up to 36 places for it until the ground takes one, the
first few for a big field and the last for one plot. A field is a grid of up to
3 by 3 plots, each side drawn from `plot-m` (6 to 24 m), set apart by lanes one
Voxel wide that also ring it. Each plot is one crop, drawn by weight from
`crops` (1 to 4 `crop` Plants without `fruit-vine`), planted every second row
ripe or at a hash-drawn Stage. The lane along the field's lowest edge is the
ditch, cut into the ground and filled with the fluid `ditch`; the other lanes
are laid in the solid Material `path`. Either may be left out. A field stands
only where the ground is level within a metre, every sampled column is in the
Biome and above the lake and caves, and no tree stands in it; a place that fails
is skipped whole. `near-water-m` (above 0, at most `spacing-m`) keeps a field to
within that distance of the lake, and a World with no lake has none. Wild plants
do not stand on a field's lanes or plots. A later entry gives way to an earlier
one it would cross. Crops stand ripe or growing like any generated crop, and the
Server grows them: a field is only where worldgen puts the first Voxels.

### Grown against placed

A Game that wants wood from trees declares two Materials of the same look: a
placed one, which has an Item, and the tree's grown trunk, which has none. The
tree's `trunk-yield` gives the placed Item. A Player builds with the placed one,
and since only grown Materials are ever part of a Felling, a cabin of placed
logs and leaves stays standing when one log is cut, however much it looks like a
tree.

## Templates and Settlements

A Template is a building declared as data: a grid of Template cells in metres,
each a Material, a Shape and an Orientation, or Keep
([ADR-0029](adr/0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md)).
A Biome lists the Templates worldgen stands in it as Structures, and the
Settlements it lays out of them; a Creative Player stamps one with the `place`
Tool. Nothing is stored per Structure: once placed it is only Voxels. The Engine
ships no Template, so a World with no Game has none.

### Templates

The hut in [A Declaration using every table](#a-declaration-using-every-table):

```toml
[[templates]]
name = "hut"
cell-m = 1.0
ground-layer = 1
foundation = "showcase:stone"
layers = [
  ["gggg", "g..g", "g..g", "gggg"],
  ["wwww", "w.ww", "wwww", "wwww"],
  ["ssss", "s..s", "s..s", "s..s"],
  ["ssss", "s..s", "s..s", "s..s"],
  ["_rr_", "rrrr", "rrrr", "_rr_"],
]

[templates.palette]
"s" = { material = "showcase:stone" }
"w" = { material = "showcase:wood" }
"r" = { material = "showcase:wood", shape = "core:slab" }
"g" = "ground"
"." = "air"
"_" = "keep"
```

`layers` run from the bottom up. Each is a list of rows, the first at the -Z
edge, and each row is a string of palette characters along +X; every layer has
the same rows and every row the same length. This hut is 4 by 5 by 4 m: a cellar
walled in the local soil, a wooden floor with a hole down into it, stone walls
with a doorway on the +Z side, and a slab roof whose corners are Keep.

A palette character is one of:

- `{ material, shape, orientation }`: a solid Material by full name, a Shape by
  full name (omitted is a full cube) and an orientation byte, `up * 4 + turn`, 0
  to 23 (omitted is 0). A Door is its closed Material and a Container is empty
  until a Player fills it. A fluid is written as a full cube with no Shape or
  orientation, and lands as a source at every Voxel size, so a well holds still
  water that neither worldgen nor the flow drains. A Grown Material and a Door's
  open Material are refused.
- `"air"`: carves, so a cellar is cut into the ground and a room into a
  hillside.
- `"keep"`: never written, so what the Structure stands in shows through.
- `"ground"`: the surface Material of the Biome the column stands in, at and
  above the Ground layer, and its soil below, as a full cube. One Template then
  sits on grass, sand or snow alike. Rock is never used, however deep a cellar
  goes.

`ground-layer` is required: the layer, counted from 0 at the bottom, that sits
at ground height. Layers below it are underground. `foundation` is a solid
Material that worldgen fills under each column, from the ground up to the lowest
layer, so a Structure on uneven ground does not float; the `place` Tool lays
none. `level-tolerance-m`, 0 to 16 m and 1 when omitted, is how far the ground
under a Structure may rise and fall. `spans = true` makes a bridge: see
[Bridges](#bridges).

`cell-m` is a Template cell's side in metres. A World takes, for each of its
Voxels, the Template cell its centre falls in, so a Template whose sides are
whole metres has the same footprint at 1.0, 0.5 and 0.25 m. A side that is not a
whole number of the World's Voxels rounds to the nearest: a 0.5 m side is one
Voxel in a 1 m World. A Template cell finer than the World loses detail, and a
coarser one repeats.

A Structure turns in quarter turns about the vertical axis; one quarter takes
the Template's +Z edge to +X, and every Template cell's Orientation turns with
it. It is never mirrored.

A Template is at most 64 m a side and 65,536 Template cells, and a World holds
at most 64 from all its Mods. Any Mod may declare one, and an unknown Material
or Shape, a character not in the palette or a grid over a limit is a load error
naming the Template and the Template cell. A component returns the same record
as a `TemplateDef`: the [Rust starter](../starters/rust/src/lib.rs) declares a
stone hut in its `hut()`. The five demo Games generate theirs from the approved
mockup with `node mockup/export-templates.mjs`, into
`games/<game>/src/templates.rs`, and list them in
`games/<game>/src/structures.rs`.

### Obtainable

Every Material any Template names, and every Settlement's `path`, must be
Obtainable in the World: a Player there can get its Item without the building.
The Engine works it out once, when a World's Game and Mods are composed:

1. Start from what worldgen lays: the World's and each Biome's ground, the lake
   fluid, ores, surface features and crop fields.
2. Add what those give: breaking a laid Material gives its Plant's yields, else
   its own Item, and a laid Plant grows every Material of that Plant.
3. Add every Recipe output whose inputs are already Obtainable, at `hand` or at
   a Station some Obtainable Material declares, until nothing changes.

Structures, the Catalogue and Mod Commands are not sources: a building is never
the only source of its own Materials. A `ground` cell is always Obtainable,
since the Biome lays it, and a fluid cell is not checked: a well is meant to be
a Settlement's water, even in a World that lays none. A Template no Biome lists
is checked too: the Creative `place` Tool stands it as a Structure. A breach
refuses the World before it opens, naming the Template, the palette character
(or `foundation`) and the Material:

```text
mod `showcase`: template 0 `hut`: palette `s`: Material `showcase:marble` is not Obtainable: no Biome lays it, no Plant yields it and no Recipe makes it from what is
```

When a Recipe's inputs are Obtainable but its Station is not, the message ends
with "at a Station a Player can get (needs `oven`)". Add a Recipe that makes it
from Obtainable Items, a Biome or ore that lays it, a Recipe for the Station, or
a dependency on a Mod that does, such as `workshop` for the shared Stations.

### Biomes stand Structures

A Biome lists at most 4 `structures` rows:

```toml
structures = [{ template = "showcase:hut", spacing-m = 64.0, chance = 0.5 }]
```

`spacing-m` is at least twice the Template's longer side and at most 4096 m, and
`chance`, above 0 and at most 1, is the share of lattice cells that try one.
[Where they stand](#where-they-stand) says how places are picked. A place is
taken only if every column of the footprint, sampled a metre apart:

- is in a Biome that lists the Template;
- is above the lake;
- has no cave cutting its top metre;
- and lies within the Template's `level-tolerance-m` of every other.

The Ground layer's top then stands on the lower median of those heights, so a
floor is level with the ground at every Voxel size, and the `foundation` fills
down to any column that falls further. A Structure's non-Keep Template cells
overwrite terrain, ore and lake water. Trees, scatter and crop fields keep off
its footprint, and a Structure never asks about them. Each Chunk works out every
Structure that can reach it and writes only its own Voxels, so a Structure is
whole across Chunk borders whichever Chunk generates first, and the same at
every Voxel size. The coarse LOD levels leave Structures out, and a Player never
spawns inside one.

A World freezes the Templates and Settlements its Biomes name in its generation
snapshot when it is created. Editing a Template later changes nothing in a saved
World, and a World saved before its Game had Templates generates none.

#### Bridges

A Template with `spans = true` is a bridge, and stands only where it crosses a
gap. Its span is its longer side after its turn, at least 6 m. Worldgen samples
it a metre apart, and a place is taken only if:

- the first and last 2 m of the span, its landings, are dry, in a Biome that
  lists it and within its `level-tolerance-m` of each other; the Ground layer
  sits on their lower median;
- under the middle third of the span, every column's ground lies at least 2 m
  below the Ground layer or is under water, and where water makes the gap it is
  at least half a metre deep somewhere;
- no ground under the span rises above the higher landing into the deck, the
  lowest Voxel cell between the landings from the Ground layer up;
- and no cave reaches under any of it.

Flat ground never takes a bridge. A random point seldom lands on a gap, so each
lattice cell tries a grid of points 2 m apart in it, with all four turns, and
keeps the deepest gap whose footprint is inside the cell; equal depths go to the
first tried, in an order from the seed and the cell. A bridge's row so skips the
neighbour check, since nothing outside its cell can touch it. The `foundation`
fills only under the landings. Between them the bridge's `air` and `ground`
cells keep whatever is there, ground, water or air, and only its Voxel cells
(deck, piers, arch) are written, over ground or water where they meet it. A
Settlement may not draw a bridge.

### Settlements

A Settlement is a hamlet, village or town declared as rules rather than drawn,
so no two are alike. The hamlet in
[A Declaration using every table](#a-declaration-using-every-table):

```toml
[[settlements]]
name = "hamlet"
radius-m = 16.0
plots = 4
minimum = 2
path = "showcase:stone"
templates = [{ template = "showcase:hut", weight = 1.0, min = 1, max = 4 }]
```

and its Biome's row:

```toml
settlements = [{ settlement = "showcase:hamlet", spacing-m = 96.0 }]
```

- `radius-m`, above 0 and at most 160, is the disc it is laid out in; every
  Template it names must be no wider than that. From 66 m its roads fork
  part-way into streets, which keep the same grade and bend rules.
- `plots-min` and `radius-min-m` make it a range: each copy draws its most Plots
  evenly from `plots-min` to `plots` by the World seed and its lattice cell, and
  its radius from `radius-min-m` to `radius-m` following the Plots drawn.
  `plots-max` and `radius-max-m` are other names for `plots` and `radius-m`.
  Omitted, the copy is a fixed size, laid out as before ranges existed. Every
  Template must then be no wider than `radius-min-m`, and `minimum` and the
  `templates` rows' `min`s count against `plots-min`.
- `margin-m`, 0 to 64 m and 16 when omitted, keeps single Structures that far
  outside the disc. Trails run on past it.
- `centre` is an optional Template stood at the centre, such as a well. It must
  pass a single Structure's ground rules, or no Settlement stands there.
- `plots`, at most 96, is how many Plots it tries to fill, drawn from
  `templates`: each Template's `min` first, then by `weight` up to `plots`,
  never past a Template's `max`.
- `minimum`: fewer Plots standing than this, or a Template's `min` unmet, and
  the Settlement is dropped. Poor ground makes a Settlement smaller before it
  makes it missing.
- `path` and `path-shape` are its Roads: a solid Material, Obtainable, and the
  Shape level Road takes.

A Biome lists at most 2 `settlements` rows, each a `spacing-m` at least twice
the largest radius and margin, so no two copies meet whatever they draw, and at
most 4096 m, and a `chance`, above 0 and at most 1 and 1 when omitted, that a
lattice cell tries one. A row may also say `near-spawn = true`: the four lattice
cells round the origin then always draw, win against their neighbours and draw
their centre clear of the spawn's keep-out, so the Settlement stands within
about a cell's diagonal of spawn wherever the ground takes it; its `spacing-m`
must then be more than its largest radius and margin plus 50 m, or the row draws
as any other. Settlements claim ground first, then single Structures, crop
fields, trees and scatter, and nothing asks back up that chain; a later row
gives way to an earlier one it would overlap.

Every Plot's door faces its Road. The door is the first closed Door Template
cell in the lowest layer, from the Ground layer up, that holds one; a Template
with none turns its +Z edge to the Road, so an open-fronted barn or a market
stall opens onto it. A Plot stands where every metre sample of its footprint and
a 2 m Apron round it is dry, no cave cuts the footprint's top metre, and every
sample is within 1.5 m of the footprint's lower median. It is then cut and
filled level, by at most 2 m, and the Apron eases back to the natural ground, so
a hillside village sits on terraces with no cliff at their edges. A Plot needs
no foundation, and the heightmap is unchanged. A Plot whose ground fails stays
empty, and a single Structure in the wild keeps its strict level rule.

The demo Games declare a hamlet of 3 to 6 Plots and a village of 8 to 20 round a
well (Tidewater's harbour stops at 12, as its islands do), and Terra also a
cobbled town of 30 to 40; see `games/<game>/src/structures.rs`.

### Loot

A chest worldgen stands holds a few Items, drawn to suit its place. A Settlement
may declare `loot`, a `templates` row may give its building its own, and a Biome
may declare `loot` for the chests of its wilderness Structures:

```toml
[settlements.loot]
rolls-min = 1
rolls-max = 3
entries = [
  { item = "crops:bread", min = 1, max = 2, weight = 3.0 },
  { item = "showcase:nail", min = 2, max = 8 },
]
```

Each roll draws one entry by weight and a count from `min` to `max`, a Stack in
a slot of its own; a roll past the last free slot is dropped. A chest in a
Settlement building draws from its row's table, else the Settlement's; a chest
in a wilderness Structure from the table of the Biome at its column. With none,
it starts empty. Every Item must be declared, but need not be Obtainable another
way: loot is a source.

The draw happens the first time a Container is needed for the chest, when it is
opened or written over, and comes from the World seed, the Structure's corner
and the chest's Template cell, so it is the same whenever it happens and at
every Voxel size. Where a Template cell is several Voxels, only its first,
lowest on every axis, draws. The cell is then looted for good: a chest a Player
puts back there, any chest a Player places, and the chests of a Structure the
`place` Tool stands, start empty. Breaking an unopened chest spills its draw.

### Where they stand

This section is the one place that says how Structures, Settlements, Plots and
Roads are spread; the rules above do not depend on it.

Each `structures` row has its own square lattice of `spacing-m` lattice cells,
salted by Biome and row. From a hash of the seed and the lattice cell alone,
every lattice cell draws a candidate (a turn and a corner on whole metres
anywhere in it) and a priority. The candidate stands, with `chance`, unless one
of the 8 neighbouring lattice cells has a higher-priority candidate nearer than
`spacing-m`. Only then are the ground rules asked; a candidate the ground
refuses leaves its lattice cell empty, with no retry, so a Game that wants more
Structures on rough ground lowers `spacing-m`. The result is spread evenly but
never on a visible grid, and differs with every seed. A later row's Structure
gives way to an earlier row's it would touch.

No Structure's footprint, and no Settlement's disc and margin, comes within 48 m
of the World origin (x = 0, z = 0), kept free for a Game that builds there. A
candidate that would is dropped like one the ground refuses. A new World's
Player spawns just outside the disc of the Settlement nearest the origin, within
512 m of it, on the side facing the origin or the nearest dry side; with none
that near, over the origin.

Each `settlements` row has a lattice of its own and the same rule: a centre
anywhere in its lattice cell stands, with `chance`, unless a neighbour's
higher-priority centre is nearer than twice the Settlement's radius plus margin.
From the centre 3 or 4 Roads branch in 4 m steps, each step the gentlest of 5
bends that stays in the disc, dry, off the centre's Plot and no steeper than
0.45 rise over run. Plots are tried along each Road on both sides in a seeded
order, at seeded gaps of 1 to 4 m and set back 0 to 2 m, and each keeps up to 4
m more clear ground the further it stands from the centre, so houses thin out
towards the edge.

Roads are graded over the Settlement's metre grid, no steeper than 0.45 rise
over run, and laid in the `path` Material on the ground. Level Road takes
`path-shape`; where it climbs it becomes the built-in slab and stair in the
`path` Material, never more than half a Voxel between neighbours, so a Player
walks up it. A stretch more than 2 m above or below its ground is not laid.

Roads narrow towards the disc's edge, and past half the radius each Road cell
wears to the Biome's surface by its own seeded draw, up to a quarter of them at
the edge. Two or three Roads run on past the disc as trails, a seeded 40 to 90
m, past the margin over ground the Settlement does not claim, so a Player finds
a path leading in. A trail follows the grade rules, stops at water, at another
Settlement's disc and margin and at the origin's keep-out, and wears away
entirely by its end so it fades into the ground. A wilderness Structure may
stand on a trail. Trees, scatter and fields keep off Roads and trails.

### The place Tool

A Game declares the Tool by name and kind:

```toml
[[tools]]
name = "place"
kind = "place"
```

Only a Creative Player can use it; in Survival it is refused. K opens the list
of the Game's Templates and takes the Tool into hand, and K or Backspace closes
it. With it held, R turns the Structure a quarter (R is also Respawn, which it
does only while the Tool is not held). Placing takes two steps: Place (right
mouse) pins a preview where the Player aims, which stays as they walk round it
(within 64 m); within 8 m of it Use (G) builds it, and K, Backspace or Remove
(left mouse) aimed at it cancels it. The preview is the Player's own and is
never saved. Its ghost shows each Template cell in its own Material,
translucent, and red where placing it would be refused; carved cells are
outlined. Only the anchor, the cell in front of the face aimed at, needs to be
in reach; the footprint's near edge sits on it, centred across your view, and
the Structure extends away from you, so it stands in front of you. Aimed
straight down, the footprint centres on the anchor. The Ground layer replaces
the layer aimed at, so aimed at the ground's top a floor and a door's sill are
level with the ground outside. Placing costs no Items and lands whole or not at
all, waiting up to 64 ticks for Chunks still loading before it refuses with a
reason. It is one undo group, and Mods hear its Voxels as Edit events. A
placement that would change more than 131,072 Voxels is refused as too big to
undo; at 0.25 m a 32 m building can.

## Artwork

A PNG is referenced by relative path from `appearance.texture`, `items.texture`
and `world.character.skin`:

| Use                | Dimensions       | Notes                             |
| ------------------ | ---------------- | --------------------------------- |
| Material `texture` | exactly 48 by 16 | Three 16 by 16 faces side by side |
| Item `texture`     | exactly 16 by 16 | One face, drawn as a flat sprite  |
| Character `skin`   | exactly 64 by 64 | Laid out by the bones' `uv` boxes |

Any PNG colour type decodes to straight-alpha RGBA. A missing, undecodable or
wrongly sized file fails before any World opens:

```text
error: mod `starter_data`: declaration: appearance[1].texture: `art/turf.png` is 16 by 16 pixels, expected 48 by 16
error: mod `starter_data`: declaration: appearance[0].texture: `art/missing.png`: No such file or directory (os error 2)
```

### Thumbnail

`thumbnail = "thumbnail.png"` in `mod.toml` names the picture menus show for a
Mod or Game: a PNG, exactly 512 by 288 pixels, at most 512 KiB, inside the
folder. It is shown before Trust and is not part of the build, so changing the
picture needs no new Trust. `package` copies it. A bad one refuses the folder:

```text
error: mod `starter_data`: mod.toml: thumbnail `thumb.png` is 256 by 144 pixels, expected 512 by 288
error: mod `starter_data`: mod.toml: thumbnail `thumb.png` is larger than 524288 bytes
```

Without one, a data Mod is drawn as one swatch per Material, coloured from its
`appearance` rows. The demo Games' and Mods' thumbnails come from
`scripts/generate-thumbnails`.

## Confinement

Every path in a Manifest or Declaration must be relative and stay inside the
Mod's own folder after symlinks are resolved. There are no network or absolute
references, so an installed Mod is self-contained:

```text
error: mod `starter_data`: declaration: appearance[0].texture: `../outside.png` must be a relative path inside the mod's directory
```

## Reading a diagnostic

An error names the Mod and the field, and `validate` exits 1 (a usage error
exits 2). These are real outputs, each from one edit to the data starter:

| Edit                               | Output                                                                                                                                              |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `emission` misspelled `emision`    | ``mod `starter_data`: declaration: TOML parse error at line 3, column 1`` then ``unknown field `emision`, expected one of `name`, `emission`, ...`` |
| `surface = "starter_data:grass"`   | ``mod `starter_data`: world: ground.surface names no registered Material``                                                                          |
| `dimming = 99`                     | ``mod `starter_data`: Material 0 `starter_data:stone`: emission and dimming are at most 15``                                                        |
| `wasm` added beside `declaration`  | ``mod `starter_data`: mod.toml: select exactly one source: wasm or declaration``                                                                    |
| `api = "0.2"` with a `declaration` | ``mod `starter_data`: mod.toml: declaration requires api 0.3``                                                                                      |
| `package` onto an existing folder  | `already exists; pass --force to replace it` (only a packaged Mod is replaced, never a project)                                                     |

## Trust and exact builds

**Trust** authorizes one exact build of a Mod. Outside development, both sources
need it: the Engine asks on first use (or take `--trust-all` /
`TESSERAE_TRUST_MODS=1` for a script) and records the build's hash. A build is
the Manifest bytes, the Declaration or component bytes, and every PNG the
Declaration references. The `thumbnail` file is not. Editor files, notes and
unreferenced images are not part of it, so they never ask again or break
anything. Changing a referenced PNG, or one byte of the Declaration, does.

A saved World requires the exact builds it was created with, dependencies
included. Reopening with a changed build, or a Mod that is gone, is refused
before anything is written, naming the Mod. The Engine never migrates a save to
new content: to keep a World, keep the build. To try a new build, create a new
World, or use `tesserae dev`, whose World is disposable.

`tesserae dev` uses **run-only Trust**: the projects named on its command line
are trusted for that process and no record is written. Dependencies that are
installed load as usual, and an installed component still needs a recorded
grant. `validate` and `package` take `--trust-for-run` for the same purpose.

## The development loop

```sh
tesserae dev my_game [more projects...] [--mods <dir>] [--seed <n>]
```

It opens a World titled "Disposable development World", and watches the named
projects (everything in them except dot-folders and `target`). Each burst of
saves is read once the files stop changing, then prepared and validated off the
main thread as one candidate through the same code as `validate`.

- **A valid change restarts.** The running session is closed by the ordinary
  teardown and a fresh World opens with the **same seed**. The Player returns to
  the Game's initial spawn. Edits you made, and simulation progress, are
  **discarded**, not carried over. A change that loads as the identical build (a
  re-save, a notes file) restarts nothing.
- **An invalid change does nothing but explain.** The running World stays, the
  terminal names the Mod and field, a repeat of the same report is printed once,
  and the next valid save recovers without restarting the command.
- **A replacement that fails while starting** leaves you in the Engine's error
  state with the reason; the next valid save, even of the same bytes, opens
  another.
- Saved Worlds are never opened, written or checked against the build, and no
  Trust record is written.

### Building before a restart

A component project enables a build in its own `dev.toml` (shown above).
`command` is a program and arguments, with no shell, run in the project folder;
`output` is a relative path inside it, copied over the `wasm` file `mod.toml`
names only when the command succeeds. Only that file enables a build: it
defaults to off, installed Mods are never built, and `package` leaves `dev.toml`
out. Enabling a build on a data-only project is refused.

- A project is rebuilt when its sources changed since its last successful build;
  the `wasm` file and `target` are not watched, so output starts no loop.
- A failed build (non-zero exit, or no `output`) names the project and prints
  the end of the diagnostics. The last good `.wasm` stays, the World keeps
  running, and the next source edit builds again.
- A build that succeeds is then validated; invalid content after a good build
  also leaves the World running.
- An edit during a build lets it finish, drops its result and builds the newer
  sources.
- **Limits.** The build has no timeout and cannot be cancelled from
  `tesserae dev`: a command that hangs holds the loop until you stop it, and the
  command runs with your user's permissions, so enable it only for code you
  trust.

## Restart, live reload and save migration

These are three different things, and only the first exists.

| Capability         | What it would do                                      | Status                                                  |
| ------------------ | ----------------------------------------------------- | ------------------------------------------------------- |
| **Restart**        | Close the World and open a fresh one with new content | `tesserae dev`, for a disposable World only             |
| **Live reload**    | Change registered content inside a running World      | Not built: a rebuilt Registry can change Material ids   |
| **Save migration** | Open an old save with new content                     | Not built, and not planned for the SDK: saves are exact |

## Contract compatibility

The rules, in full, are in
[`contract-compatibility.md`](contract-compatibility.md). In short: a patch
(`0.3.0` to `0.3.1`) is compatible, a closed record or variant that changes is a
new line (`0.4`), the Engine keeps hosting older lines through adapters, and
moving a Game to a newer line means rebuilding it against that line's bundle.

Line `0.x` is pre-release. This guide documents how contracts evolve; it does
**not** claim the 1.0 contract is frozen. That freeze belongs to roadmap step
27, after the gameplay systems that will add to the contract exist.

## What has been checked

- The repository's tests copy each starter outside the checkout and run the
  steps above: `validate`, `package`, loading the installed result, a saved
  World reopening unchanged and being refused after a build change, Declaration
  and PNG edits through the development loop, and, for Rust, the README's build,
  its `dev.toml` through a real Cargo rebuild, a compile error and its fix. The
  full-schema example above is extracted from this page and validated.
- Not checked automatically: the running window. That a restart works on a real
  display was checked by hand and is recorded in ticket 08 of the Modding SDK
  spec.
- If `CARGO_TARGET_DIR` is set, Cargo builds outside the project and `output` in
  `dev.toml` is not found; unset it for `tesserae dev`, or point `output` at the
  real path.

### Non-Voxel Causes and Biome audio

Only the Game declares `cause-sounds`: each row binds `cause` to a Sound's full
name in `sound`. The closed Causes are `splash`, `burn`, `swimming`, `floating`,
`pickup`, `use-measured`, `use-refused`, `eat`, `hurt`, `death`, `door-open`,
`door-close`, `container-open`, `container-close` and `craft`. Unbound Causes
are silent; duplicate bindings and non-Effects Sounds are refused. The Server
emits these events. `door-open` and `door-close` sound at the Door,
`container-open` and `container-close` at the Container, and `craft` at the
Player, once per landed action and never for a refusal.

A Biome may set `soundscape` to a loaded Soundscape's full name, including one
from a data-only Mod without a context. Unknown names are refused. The Eye
selects underwater, underground, its Biome, then default, falling through
missing contexts. Leaving `soundscape` out preserves the default selection.

The unreleased 0.3 `sound-cause` enum also includes `swimming` and `floating`.
The Game binds them through `cause-sounds`, like `splash`. Swimming strokes
require roughly 3 m of travel and a seeded 1.6–2.2 s cooldown, including while
moving at the surface. Gentle idle floating sounds occur at seeded 6–12 s
intervals. Neither is emitted while grounded, flying, dead or paused. Unbound
Causes remain silent.

Audio preferences include per-action trims for footsteps, landings, breaking and
striking, placing, eating, swimming, floating water, pickups, tools, and
hurt/death. They multiply the Effects and Master settings and apply to playing
sounds on Apply. Older profiles default every trim to 100. These preferences add
no Mod contract fields. Ambient recordings have a separate 4 MiB ceiling;
Effects keep 512 KiB, Music 8 MiB and referenced World audio 32 MiB.

Continuous damage applies every simulation tick, while `hurt` feedback is
limited to one Sound every 0.75 s to prevent overlapping short clips. Death
emits once at the health transition; it is not a repeating damage Sound. The
damage-feedback interval changes neither damage amounts nor hazard rules.

The unreleased 0.3 `sound-cause` enum appends `burn`: the Server emits it once
when the body enters a fluid whose declaration has `heat`, including a direct
cold-to-hot fluid transition. Bind it through Game `cause-sounds`, including in
data declarations; an unbound Cause is silent. Cold-fluid entry/exit uses
`splash`; hot-fluid exit is quiet. Hot fluids do not emit `swimming`/`floating`
or select the underwater water Soundscape. Burn uses the Vitals detail trim.
Health damage and hurt/death timing are unchanged. Older 0.1/0.2 declarations
retain their existing adaptation without Cause bindings.
