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

The complete field list, with a Declaration that uses every table, is in the
[authoring guide](authoring-guide.md#the-declaration-schema). Only Games may
declare `world`, `view` and `tools`, except that a Mod may supply
`world.character` and nothing else of `world`
([ADR-0021](adr/0021-a-mod-may-supply-the-character.md)); a World has one
character source, and two fail the load naming both.

Any Mod may add `[[biomes]]` and `[[ores]]` to the Game's World;
`[world.climate]` is the Game's alone. Biomes and ores append in load order
(dependencies first), so a Biome's id is its place in that order, and the World
holds at most 16 Biomes and 32 ores. A Biome row is `name`, `temperature` and
`humidity`, the last two from 0 to 1; its full name is `<mod id>:<name>`, at
most 32 bytes. An ore row is `material` (a solid Material, by full name),
`min-depth-m` and `max-depth-m` (the band below the surface, shallow end first),
`vein-size-m`, `rarity` (above 0, at most 1) and an optional `biomes` list of
full Biome names from any Mod; omitted, the ore is in every Biome.
`[world.climate]` is `temperature-size-m` and `humidity-size-m`, in metres. A
Biome row may add `ground = { rock, soil, surface }`, solid Materials by full
name from any Mod, checked as `[world.ground]` is. A column takes the ground of
its Biome, switching hard at the border, and a Biome without `ground` has the
World's; surface height, relief, sea level and lakes are never a Biome's. A
World with no ground is empty air whatever its Biomes name.

An ore is a blob of `vein-size-m` metres at most, in a cube of that edge; a
`rarity` fraction of the cubes hold one. It replaces rock only, never soil, the
surface, water or air, inside its depth band and its Biomes. Caves carve the ore
with the rock, the farthest LOD levels carry none, and a World on Generator
version 1 has none.

Tables that are absent change nothing: a World with no Biomes generates as it
always did.

A combustion `end` is `"permanent"` or
`{ after = { seconds = 8, residue = "mod:ash" } }`; `residue` omitted removes
the solid. Fluid, reaction, combustion, ground and water names are full
`<mod id>:<name>` names and resolve after every Mod has declared, in dependency
order, whether their Material came from data or a component. Data-only Mods have
no guest instances, `start` or `tick` callbacks. Dependencies use the existing
`depends` Manifest field and load before their consumers, regardless of
Declaration format. Paths must be relative and stay inside the installed
directory after symlink resolution.

A Game's Manifest may list `suggests = ["wanderer"]`: Create World ticks those
Mods when the Game is picked, but they can be unticked or swapped, unlike
`depends`, which is locked on. Nothing loads a suggestion unless the World
records it.

A Manifest may also carry two display-only fields, which Create World uses to
list a Mod under a section with a line about it: `category` (1 to 24 characters,
for example `"Fire & light"`) and `description` (1 to 80 characters). Neither
changes what the Mod does, but both are part of the Manifest, so editing one
asks for Trust again like any other change.

## Validating and packaging projects

Both commands run without opening a World and take project folders (each holds a
`mod.toml`), wherever they are:

```sh
tesserae validate <project>... [--mods <dir>] [--trust-for-run]
tesserae package <project>... --out <dir> [--mods <dir>] [--trust-for-run] [--force]
```

`--mods` is the installed `mods` folder (default `TESSERAE_MODS_DIR`, then the
user data directory). It supplies dependencies the projects do not include, and
the `games` folder beside it supplies a Game, as at startup. A project shadows
an installed item of its id.

Validation loads the projects and their dependencies through the code a World
uses: Manifest, dependency order, Trust, Declaration (data or component),
artwork and name resolution. It stops before any component's `start`, so no
World exists. It prints one `ok:` line per Mod, or exits 1 with an error naming
the Mod and the field or file; a usage error exits 2.

Trust is unchanged. A data Declaration runs no code, so it needs no grant to be
checked. A component needs a recorded grant, or `--trust-for-run` for the
projects named on the command line (never for installed dependencies); the flag
writes nothing.

`package` writes `games/<id>/` or `mods/<id>/` under `--out`, holding exactly
`mod.toml`, the Declaration or component, and the referenced PNGs, at the same
relative paths. Source, development configuration and unreferenced files are
left out. The bytes written are the bytes that were checked; the copy is then
loaded again, in place of the source, and must have the same build before
anything is installed, so a project that changes or fails during the run
produces no output. An existing folder needs `--force`, and only a packaged Mod
is replaced. Select the result with `--mods <dir>/mods`.

## Watching a project

```sh
tesserae dev <project>... [--mods <dir>] [--seed <n>]
```

Opens a disposable World ("Disposable development World" in the window title and
terminal) from the named Game and Mods, and watches them. Nothing is saved and
no Trust record is written: the named projects are trusted for this run only,
while installed dependencies load as usual and an installed component still
needs a recorded grant. `--seed` fixes the seed (otherwise the Engine's
default). No build command runs unless a project enables one (see below);
otherwise rebuild a component yourself and the new `.wasm` is picked up like any
other file.

Each saved change is prepared and validated off the main thread through the same
code as `validate`, as one consistent candidate:

- A valid candidate closes the running session by the ordinary teardown and
  opens a fresh World with the same seed. The Player returns to the Game's
  initial spawn; Edits and simulation progress are discarded, not migrated. A
  change that loads as the identical build (an editor file, a re-save of the
  same bytes) restarts nothing.
- An invalid candidate (malformed data, missing, invalid or wrongly sized
  artwork, rejected content, an untrusted installed component) leaves the World
  running and prints the Mod and field. Repeated identical reports are printed
  once. The next valid save recovers without restarting the command.
- A burst of saves is read once after the files stop changing; a candidate that
  was prepared while a newer edit landed is dropped and the newer state
  prepared, so a stale version never replaces a newer one.
- If the replacement World stops while starting, the session ends in the
  ordinary error state with the reason, and the next valid save, even of the
  same bytes, opens another.

### Building before a restart

A component project opts in to automatic building with a `dev.toml` beside its
`mod.toml`:

```toml
[build]
enabled = true
command = ["cargo", "build", "--target", "wasm32-wasip2", "--release"]
output = "target/wasm32-wasip2/release/my_game.wasm"
```

`command` is a program and arguments (no shell), run in the project folder.
`output` is the component it produces, a relative path inside the project; on
success it is copied over the `wasm` file `mod.toml` names. Only this file
enables a build: `enabled` defaults to false, installed Mods are never built,
`package` leaves `dev.toml` out, and a project without one, or a data-only
project, is only loaded. Enabling a build on a project whose `mod.toml` has no
`wasm`, or a malformed `dev.toml`, is refused naming the project.

The build is the first step of preparing a candidate, so the rules above still
hold:

- Each enabled project is built when its sources changed since its last
  successful build (files other than dot-folders, `target` and the `wasm` file
  the build writes, which are not watched, so generated output starts no further
  build). A change to a data-only project or to a project without a build does
  not rebuild the others.
- A failed build (non-zero exit, or no `output`) is refused naming the project
  and printing the end of the build's diagnostics. The `wasm` file on disk is
  not replaced, the current World keeps running, and the next source edit builds
  again.
- A build that succeeds is then validated like any candidate; invalid content
  after a successful build also leaves the World running.
- An edit that lands during a build lets that build finish, drops its result and
  builds the newer sources, so an older output never replaces a newer one.

The build runs with your user's permissions, so enable it only for projects you
trust.

Only the named projects are watched (everything inside them except dot-folders
and `target`); other local projects and installed Mods are not. Saved Worlds are
never opened, written or checked against the build.

An item row is `name`, `size-m` (the dropped cube's edge, 0.05 to 0.5 m),
`color` and an optional `material`: a solid Material, `<mod id>:<name>`, whose
removed Voxels drop it. Any Mod may declare items, at most 64 per Mod and 256 in
a World; two items may not name one Material. Lines 0.1 and 0.2 declare none.
