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

Material rows require the six fields above and accept optional `remove-time-s`
and `contact-damage-per-s`. `occlusion` is `opaque`, `same-material` or `never`;
`render` is `opaque`, `blend` or `cutout`. Appearance colours are four RGBA
bytes; Material names in appearance are local, while ground names include the
Mod id. Both formats use the same content bounds and validation.

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
declare `world`, `view`, `tools` and `survival`, except that a Mod may supply
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

A Biome row may also add `sky-fog = { sky = [r, g, b], fog = [r, g, b] }`, sRGB
bytes, both together or neither. `sky` is the colour behind the terrain and
`fog` is what distant terrain fades toward, a little lighter than `sky` because
the tonemapper darkens the fog on its way to the screen. The Server blends the
pairs of the Biomes near the Eye by closeness in Climate, so the air changes
smoothly and never in a step, and sends the Client the pair when it has moved a
visible step; the Client eases to it, sky and fog together, and underwater fog
still wins below a surface. A Biome without `sky-fog` has no say in the blend,
and a World where none has one keeps the Look pack's haze.

A Biome row may also add `cave-scale`, a multiplier from 0 to 4 on the World's
`caves.radius_m`: 2 carves much more rock with wider tunnels, 0.5 less, and 0
switches caves off in that Biome. Omitted, the Biome keeps the World's caves.
Unlike `ground`, it blends smoothly across a Biome's border, so there is no wall
where a cave field meets a solid one. It does nothing in a World without
`[world.caves]`, and caves still only turn solid into air.

A Biome row may also add `surface-features`, at most four
`{ material, spacing-m }` entries: a solid Material by full name from any Mod,
and the mean distance between two of them in metres, from 4 to 1024. The Engine
stands one, deterministically from the World seed, about once in every
`spacing-m` by `spacing-m` metres of the Biome's surface at any Voxel size, one
Voxel above a column whose top is a full Voxel of the Biome's surface Material
with air above it: never under water or another fluid, on a smoothed stair or
slab, or in a cave mouth. Removing one drops its Item like any Voxel. The coarse
LOD levels leave them out. The World's saved Biome table carries them, so a
World saved before a Game added features keeps its bare ground.

Tables that are absent change nothing: a World with no Biomes generates as it
always did.

Any Mod may declare `[[sounds]]`, `[[sound-sets]]` and `[[material-sounds]]`. A
Sound row is `name`, `category` (`effects`, `ambient` or `music`), `recordings`
(one or more `{ file, weight }`, each `file` an Ogg Vorbis path inside the Mod
directory, read by the same rules as artwork), `volume` and `pitch` (each
`{ min, max }`, drawn from each time it plays; pitch is a playback speed) and,
for `effects` only, `distance = { reference-m, max-m }`: whole within
`reference-m`, fading to silence at `max-m`, in metres at any Voxel size. An
`effects` Sound is heard from where it happens, so each recording must be mono;
ambient and music may be stereo, and a `distance` on them is refused. A Sound
set row is a `name` and an optional `effects` Sound, by full name, for each
Voxel Cause: `break`, `place`, `hit`, `step` and `land`; a Cause left out is
silent. A `material-sounds` row gives a Material, by full name, a Sound set, by
full name, from any Mod, so a sound-only Mod can voice a Game; a Material has at
most one Sound set in a World, and a second fails the load naming both Mods.
Names resolve after every Mod has declared. A recording that is not Ogg Vorbis,
a stereo `effects` recording, an unknown name or an ambient Sound in a set slot
fails before any World opens, naming the Mod and field.

Sounds have limits, each refused naming the Mod and field. A Mod declares at
most 64 Sounds and 16 Sound sets; a Sound holds at most 8 recordings. An
`effects` recording is at most 512 KiB, an `ambient` one 4 MiB, and a `music`
one 8 MiB, and a file over its limit is refused from its size, before it is
read. A World holds at most 32 MiB of recordings across all its Mods, each
reference counted, so a file named twice counts twice. A recording has 1 or 2
channels at 8 to 96 kHz. `weight` is 1 to 16; `volume` lies in 0 to 1 and
`pitch` in 0.5 to 2, each with `min` not above `max`; `reference-m` is 0.25 to
16 and `max-m` above it and at most 128.

Referenced recordings join build identity like artwork: changing one needs Trust
again and breaks exact-build reopening of saved Worlds, and unreferenced audio
files do not.

Any Mod may declare `[[soundscapes]]`: what is heard around the Player rather
than at a point. A row is `name`, optional `context` (`default`, `underground`
or `underwater`), `beds` (at most 8 `{ sound, volume }`, each an `ambient` Sound
by full name looped at `volume`, 0 to 1), `music` (at most 8 `music` Sounds by
full name) and optional `silence-s = { min, max }`, the silence before each
track in seconds, from 0 to 3600 and 60 to 180 when absent. Only the Game's
Soundscapes may carry a `context`, at most one for each; a Mod's exists to be
named by a Biome. The Server picks the Soundscape for where the Player's Eye is:
`underwater` while the Eye is inside a fluid, else `underground` while the Eye's
Voxel receives no sky light, else `default`, passing over a context the Game
gave none, so a Game with only a `default` hears it everywhere and one with none
is silent. On a change the old beds fade out and the new fade in over 2 s, and
the playing track fades out over 4 s while the new Soundscape's silence begins.
After each silence one track plays, never the one just heard while there are two
or more. Ambient and music play at the Player's Ambient and Music volumes, never
by distance. A Sound of the wrong category as a bed or track, an unknown name, a
`context` from a Mod or a second one for a context fails the load naming the
Mod, the Soundscape and the field.

```toml
[[soundscapes]]
name = "open"
context = "default"
beds = [{ sound = "plain:wind", volume = 0.6 }]
music = ["plain:morning", "plain:dusk"]
silence-s = { min = 90.0, max = 240.0 }
```

The Server makes a `break` when a Player's own Remove takes a solid out of the
World, never for undo, redo, a Layout, fluid, fire or a Mod's Commands, and only
when the Player is within the Sound's `max-m`.

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
`mod.toml`, the Declaration or component, and the referenced PNGs and
recordings, at the same relative paths. Source, development configuration and
unreferenced files are left out. The bytes written are the bytes that were
checked; the copy is then loaded again, in place of the source, and must have
the same build before anything is installed, so a project that changes or fails
during the run produces no output. An existing folder needs `--force`, and only
a packaged Mod is replaced. Select the result with `--mods <dir>/mods`.

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
a World; two items may not name one Material. Lines 0.1 and 0.2 declare none. An
item may also set `max-stack`, the most one Inventory slot holds (1 to 64,
omitted is 64), and `food`, the hunger eating one restores.

A `[[voxel-uses]]` row makes a Material a Usable Voxel, which Use acts on. Its
`kind` is `door`, with a closed `material` and an `open` one, both solid
Materials by full name; bodies pass through the open one, and removing it drops
the closed one's Item. Or it is `station`, with a solid `material` and a `tag`:
Use opens the Recipes whose `station` is that tag, craftable while the Player
stays in reach. Materials may share a tag, so another Mod's bench opens the same
Recipes; `hand` is reserved. Any Mod may declare them, at most 64 per Mod, and a
Material is in at most one row, as either state.

```toml
[[voxel-uses]]
kind = "door"
material = "plain:door"
open = "plain:door_open"

[[voxel-uses]]
kind = "station"
material = "plain:workbench"
tag = "workbench"
```

### Non-Voxel Causes and Biome audio

Only the Game declares `cause-sounds`: each row binds `cause` to a Sound's full
name in `sound`. The closed Causes are `splash`, `pickup`, `use-measured`,
`use-refused`, `eat`, `hurt` and `death`. Unbound Causes are silent; duplicate
bindings and non-Effects Sounds are refused. The Server emits these events.

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
