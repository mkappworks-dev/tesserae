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

Only a Game may declare `world`, `view` and `tools`.

| Table                       | Fields                                                                                                                                                                   |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `[[materials]]`             | `name`, `emission` (0 to 15), `dimming` (0 to 15), `occlusion`, `render`, `solid`, optional `remove-time-s`                                                              |
| `[[shapes]]`                | `name`, `boxes`, optional `collision`, optional `connect`                                                                                                                |
| box in `boxes`              | `min`, `max`: three integers, sixteenths of a Voxel                                                                                                                      |
| box in `collision`          | `min`, `max`: three signed integers, sixteenths of a Voxel                                                                                                               |
| `[shapes.connect]`          | `group`, `connects-to`, `to-sealed`, `arms` (each: `face`, `boxes`, optional `collision`)                                                                                |
| `[[appearance]]`            | `material` (a local name), `color` (four RGBA bytes), optional `texture` (a PNG path)                                                                                    |
| `[world]` (Game)            | optional `sea-level-m`, `relief-m`, `feature-size-m`, `soil-depth-m`, `octaves`, and the tables below                                                                    |
| `[world.ground]`            | `rock`, `soil`, `surface`: full `<mod id>:<name>` Material names                                                                                                         |
| `[world.caves]`             | `feature-size-m`, `radius-m`, `floor-m`, `octaves`                                                                                                                       |
| `[world.smoothing]`         | `min-rise-m`                                                                                                                                                             |
| `[world.water]`             | `level-m`, `material` (a fluid, by full name)                                                                                                                            |
| `[world.movement]`          | `allow-flight`, `allow-physics`, `start-in-flight`                                                                                                                       |
| `[world.character]`         | `pixel-m`, `skin` (a 64 by 64 PNG path), `idle`, `walk`, `sprint`, `airborne`, `crouch` (each `swing-deg`, `rate-hz`, `arm-raise-deg`), `bones`                          |
| `[[world.character.bones]]` | `name`, optional `parent`, `role`, `pivot`, `cubes` (each `origin`, `size`, `uv`)                                                                                        |
| `[view]` (Game)             | `view-distance-m`, `lod-distance-m`                                                                                                                                      |
| `[[fluids]]`                | `material`, `reach-m`, `sideways-m-per-s`, `downward-m-per-s`, `recession-m-per-s`, `reset-drop-m`, `waterlogs`, `extinguishes`, optional `heat` (`strength`, `reach-m`) |
| `[[reactions]]`             | `first`, `second` (each `fluid`, `state`), `consumed`, optional `product`, optional `receiving`                                                                          |
| `[[combustion]]`            | `fuel`, `ignition-threshold`, `ignition-delay-s`, `emission`, optional `heat`, `end`                                                                                     |
| `[[tools]]` (Game)          | `name`, `kind`                                                                                                                                                           |

Variants: `occlusion` is `opaque`, `same-material` or `never`; `render` is
`opaque`, `blend` or `cutout`; a bone `role` is `body`, `head`, `arm-left`,
`arm-right`, `leg-left`, `leg-right` or `fixed`; a reactant `state` is `any`,
`source` or `flowing`; `consumed` and `receiving` are `first` or `second`; a
tool `kind` is `measure`. A combustion `end` is `"permanent"` or
`{ after = { seconds = 8, residue = "mod:ash" } }`, and an omitted `residue`
removes the solid.

Materials and Shapes are declared by local name. Ground, water, fluid, reaction
and combustion names are full `<mod id>:<name>` names and resolve after every
Mod has declared, whether the Material came from data or from a component.

### A Declaration using every table

This Game is checked by the repository's tests: it is extracted from this page,
given the two PNGs it names, and validated.

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
```

Its `mod.toml` is the data-only starter's with `id = "showcase"`.

## Artwork

A PNG is referenced by relative path from `appearance.texture` and
`world.character.skin`:

| Use                | Dimensions       | Notes                             |
| ------------------ | ---------------- | --------------------------------- |
| Material `texture` | exactly 48 by 16 | Three 16 by 16 faces side by side |
| Character `skin`   | exactly 64 by 64 | Laid out by the bones' `uv` boxes |

Any PNG colour type decodes to straight-alpha RGBA. A missing, undecodable or
wrongly sized file fails before any World opens:

```text
error: mod `starter_data`: declaration: appearance[1].texture: `art/turf.png` is 16 by 16 pixels, expected 48 by 16
error: mod `starter_data`: declaration: appearance[0].texture: `art/missing.png`: No such file or directory (os error 2)
```

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
Declaration references. Editor files, notes and unreferenced images are not part
of it, so they never ask again or break anything. Changing a referenced PNG, or
one byte of the Declaration, does.

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
- Not checked automatically: the running window. That the title, input and
  rendering survive a restart on a real display is a manual check, recorded in
  the ticket that tracks it.
