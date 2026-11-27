# Contract compatibility

The Mod contract is the WIT bundle `tesserae:mods@<line>.<patch>`, published
under `crates/tesserae-mods/wit/<line>/` and copied into
[`starters/rust/wit/`](../starters/rust/wit/). A Manifest's `api` field names
the **line** (`0.3`), never a patch.

## Patches and lines

- **A patch (`0.3.0` to `0.3.1`) is compatible.** It may change documentation,
  comments and clarifications of existing behaviour. A component built against
  any `0.3.x` loads on every Engine that supports line `0.3`.
- **A new line (`0.3` to `0.4`) is breaking.** Any change to a closed record or
  variant, a removed or retyped field, or a changed function signature is a new
  line. Adding a field to a record counts: records are closed, so the layout
  changes.
- The Engine keeps hosting older lines through adapters that map them to the
  current one, under their existing restrictions. A component or Declaration for
  line `0.1`, `0.2` or `0.3` keeps loading until a release announces that line's
  end. Data Declarations exist only on `0.3`.
- A component exports one line. Moving a Game to a newer line means rebuilding
  it against that line's bundle; the Engine never rewrites a component.

## Loader rules are not the contract

A change to what the loader accepts without changing a WIT record is not a new
line. [ADR-0021](adr/0021-a-mod-may-supply-the-character.md) lets a Mod supply
`world.character` on line 0.3: nothing that loaded before behaves differently.

## Amended before release

Line 0.3 gained `items` on its `declaration` while unreleased. A Declaration for
0.1 or 0.2 loads with no items.

Line 0.3 gained `biomes` and `ores` on its `declaration`, and `climate` on its
`world-params`, in the same way. A Declaration for 0.1 or 0.2 loads with none. A
`biome-def` carries an optional `ground`, with the same fields as the World's,
and an optional `sky-fog`, a `sky-fog` record of two sRGB colours, and an
optional `cave-scale`, a multiplier from 0 to 4 on the World's cave radius.

Line 0.3 also gained an optional-in-effect `tags` list on `material-def` (an
adapted 0.1 or 0.2 Declaration has none), so Plants can name `#soil` or `#water`
instead of a Material.

Line 0.3 also gained Marker kinds: a `marker-kinds` list on `declaration`, from
any Mod and at most 16 rows per Mod, each a `name`, an `icon` and a `color`. The
`icon` is one of the closed `marker-icon` names the Engine draws as shapes
(`home`, `camp`, `mine`, `farm`, `landmark`, `death`, `flag`, `pin`, `star`),
not a glyph. An optional `death-marker-kind` on `declaration`, from the Game
only, names the full `<mod id>:<name>` of the kind a death leaves; it must name
a declared Marker kind, resolved after every Mod has declared. It is data only:
no Mod is called and no host call is added. A Declaration for 0.1 or 0.2 loads
with none, and a World with no Game has none.

Line 0.3 also gained a `map-policy` record, an optional `map-policy` field of
`world-params` holding one `enabled` flag, from the Game only: a Mod that is not
the Game and sets it is refused. Absent permits the Map and the Compass, and so
does a World with no Game; `enabled = false` forbids them, so the Server stops
scanning and refuses Waypoint intents, and the Client draws no Compass or
minimap and opens no Map. A saved Map is kept and returns when a later load
permits it. A Declaration for 0.1 or 0.2 loads with none. It is data only: no
Mod is called and no host call is added.

Line 0.3 gained what Survival needs in the same way: an optional
`contact-damage-per-s` on `material-def`, from 0 to 1000 health per second; an
optional `max-stack` on `item-def`, from 1 to 64 and 64 when absent, and an
optional `food`, the hunger one restores, above 0 and at most 1000; and an
optional `survival` record on `declaration`, from the Game only, so a Mod that
declares one is refused. Every `survival-params` field is optional and an absent
one keeps the engine's default. A Declaration for 0.1 or 0.2 loads with none of
them: harmless Materials, Items that stack to 64 and are not food, and the
default Survival rules.

`biome-def` then gained `surface-features`, appended in the same way: a list of
at most 16 `surface-feature` records (four before the Plant phase raised it),
each a solid Material by full name and a mean `spacing-m` from 1 to 1024 metres
(4 before ticket 19), that the Engine stands one Voxel above the Biome's
surface. A Biome with none generates exactly as before.

`surface-feature` then gained `clump`, an optional `{ scale-m, strength }`
record, appended in the same way, that gathers the feature into groves with
clearings while `spacing-m` stays its mean spacing. A Declaration without it
loads and generates as before. A guest built against the earlier record must be
rebuilt, since a record's field count is part of its type.

`survival-params` then gained `regeneration-per-s`, appended in the same way:
the health a Player regains per second while hunger is at or above
`regeneration-hunger`, from 0 to 1000, and 0.25 when absent, the rate the engine
used before the Game could declare it.

Line 0.3 then gained `sounds`, `sound-sets` and `material-sounds` on its
`declaration`, appended in the same way. A Declaration for 0.1 or 0.2 loads with
none of them, so its World is silent. A `sound-def` carries its recordings as
Ogg Vorbis bytes, each with a weight, a `volume` and a `pitch` `sound-range`, a
closed `sound-category` of `effects`, `ambient` or `music`, and a `distance` in
metres that `effects` requires and the others refuse. A `sound-set-def` names an
`effects` Sound for each of `break`, `place`, `hit`, `step` and `land`, and a
`material-sound` gives a Material a Sound set, both by full name.

Line 0.3 then gained `soundscapes` on its `declaration`, appended after
`material-sounds` in the same way; a Declaration for 0.1 or 0.2 loads with none.
A `soundscape-def` carries a name, an optional closed `soundscape-context` of
`default`, `underground` or `underwater` that only the Game may set and at most
once each, up to 8 `ambient-bed`s (an `ambient` Sound by full name and a volume
from 0 to 1), up to 8 `music` Sounds by full name, and an optional `silence-s`
`sound-range` from 0 to 3600 seconds, 60 to 180 when absent.

Line 0.3 then gained `recipes` on its `declaration`, appended after
`cause-sounds` in the same way; a Declaration for 0.1 or 0.2 loads with none. A
`recipe-def` carries 1 to 9 `inputs` and one `output`, each a `recipe-stack` of
an Item by full name and a count, the output's at most its Item's `max-stack`,
and a required `station`: `hand` or a Station's tag. Names resolve after every
Mod has declared, implicit Items included. A Mod declares at most 256 and a
World holds at most 1024. Line 0.3 then gained `voxel-uses` on its
`declaration`, appended after `recipes` in the same way; a Declaration for 0.1
or 0.2 loads with none. A `voxel-use` names a solid Material by full name and a
closed `voxel-use-kind`: `door` with a `door-def`, its open Material's full
name, through which bodies pass but rays do not, and an optional `width-m` and
`height-m`, a size in metres from 0.5 to 4 wide and 1 to 4 tall, 1 by 2 when
absent; `container` with an optional slot count from 1 to 54, 27 when absent; or
`station` with a tag, which Recipes name in `station`; Materials may share a
tag, and `hand` is reserved. A Material is in at most one row, as its `material`
or as a Door's open one. A `voxel-use` then gained an optional `shape`, a Shape
by full name that a Player's Place of its Material always uses; a Door's is
drawn closed against its own +Z and turned a quarter about its hinge by Use.

`sound-cause` then gained `door-open`, `door-close`, `container-open`,
`container-close` and `craft`, appended after `burn`. A Game binds them in
`cause-sounds` like any Cause; one that binds none plays them silent.

Line 0.3 then gained `plants` on its `declaration`, appended after `voxel-uses`
in the same way; a Declaration for 0.1 or 0.2 loads with none. A `plant-def`
carries a `name` (local to the Mod), a `seed` Item by full name, an optional
`shape` (a Shape by full name for its single-Voxel Stages), a `plant-needs`
record (`soil`, optional `light-min` and `light-max`, `temperature` and
`humidity` ranges and a `water` reach) and a closed `plant-kind`: `crop`,
`tree`, `column`, `spreader` or `climber`, each with its own record. A Mod
declares at most 128 and a World holds at most 512. A `plant-yield` is an Item
and a `count-range`. A `tree-def` may carry a `fruit-def` (stages, density,
`regrow-seconds`, shape; fruit is broken to harvest and the tree grows new
fruit, so it has no `harvest-to`) and `vines`, a `tree-vines` naming a `climber`
that only worldgen hangs, an optional `branching-def` (limbs, start, reach,
rise, fork, lean, tiers and lobe radius, in metres and fractions, appended last
and absent in every earlier tree), and a `column-def` may name the fluid it
stands in. Names and `#tag` references resolve after every Mod has declared; an
unknown or unmatched one is a load error naming the Plant.

A `crop-def` of line 0.3 gained an optional `fruit-vine` (a `fruit-vine-def`:
stages, `regrow-seconds`, shape and optional `reach-m`, `gap-m` and `max-fruit`)
and a `spreader-def` an optional `min-neighbours`, both appended last and absent
in every earlier row. Both are optional fields of the unreleased text, so a
Declaration built against the earlier 0.3 text no longer lines up and needs a
rebuild; a row that sets neither behaves as before.

A `surface-feature` of line 0.3 then gained an optional `field` (a `field-def`:
`group`, `scale-m`, `strays`) and a `biome-def` a `crop-fields` list (a
`crop-field` record: crops with weights, `plot-m`, `spacing-m`, and optional
`path`, `ditch` and `near-water-m`), appended in the same way, so a Declaration
built against the earlier 0.3 text needs a rebuild. A Biome with neither
generates exactly as before, and the Biome snapshot carries them as optional
members, so no `FORMAT_VERSION` or generator version changes.

A `plant-stage` of line 0.3 gained an optional `shape` as its last field (a
Shape by full name the Stage's Voxels take; none keeps the Plant's or the
fruit's), so a Declaration built against the earlier 0.3 text no longer lines up
and needs a rebuild; an unknown name is a load error naming the Plant and the
Stage.

Four more appended fields go with them, each absent in an adapted 0.1 or 0.2
Declaration. `shape-def` gained `form`, a closed `shape-form` of `boxes`,
`cross` or `plane`, and none is `boxes`. `item-def` gained an optional
`texture`, one 16 by 16 RGBA texture of 1024 bytes, that draws an Item as a flat
sprite. `material-def` gained `tags`, above. `surface-feature` now takes an
optional `material` and an optional `plant`, exactly one set, and an optional
`shape`; the 0.1 and 0.2 adapters set `material` and neither of the others. A
`biome-def` may list 16 `surface-features`, and a Mod 128 Materials, Shapes or
appearance rows.

Line 0.3 then gained `templates` on its `declaration`, appended after
`death-marker-kind` in the same way; a Declaration for 0.1 or 0.2 loads with
none. A `template-def` carries a `name` (local to the Mod; the engine adds
`<mod id>:`), a `cell-m` grid size in metres, a `palette` of
`template-palette-entry` rows each mapping one `char` to a closed
`template-cell` (`keep`, `air`, or a `template-voxel` of a Material and an
optional Shape by full name and an optional orientation byte), `layers` of rows
of palette characters from the bottom up, an optional `ground-layer` that a
Template must set, an optional `foundation` Material and an optional
`level-tolerance-m`, from 0 to 16 and 1 when absent. Any Mod may declare one,
and a World holds at most 64. A side is at most 64 metres and a Template at most
65,536 cells. Names resolve after every Mod has declared; an unknown Material or
Shape, a fluid, a character not in the palette or a grid out of bounds is a load
error naming the Template. It is data only: no Mod is called and no host call is
added.

A `template-def` of line 0.3 then gained `spans`, a bool after
`level-tolerance-m`, false in every Declaration built before it. A Template that
spans stands only where its longer side crosses a gap, a valley or water,
between two dry landings. A span under 6 m is a load error naming the Template,
and a Settlement that draws one is a load error naming the Settlement. A
Declaration built against the earlier 0.3 text needs a rebuild; a data Mod's
`spans` defaults to false.

A `biome-def` of line 0.3 then gained a `structures` list before `soundscape`,
at most 4 `biome-structure` rows of a `template` by full name, a `spacing-m` at
least twice the Template's longer side and at most 4096 m, and a `chance` above
0 and at most 1. A Declaration built against the earlier 0.3 text needs a
rebuild. A fifth row, an unknown Template or a spacing under its floor is a load
error naming the Biome. A Biome with none generates exactly as before, with no
generator version change.

`template-cell` then gained `ground`, the surface Material of the Biome a
Structure's column stands in at and above the ground layer and its soil below,
so one Template fits grass, sand or snow. A Declaration built against the
earlier 0.3 text needs a rebuild. It is never refused as not Obtainable.

The closed `tool-kind` of line 0.3 then gained `place`, after `measure`. A Game
declares a `place` Tool by name and kind like any Tool; its Use edits nothing
and enters no history, and only a Creative Player may place a Structure with it.
A Declaration built against the earlier 0.3 text needs a rebuild.

The `declaration` of line 0.3 then gained `settlements`, appended after
`templates`, and a `biome-def` gained a `settlements` list after `structures`. A
`settlement-def` carries a `name` (local to the Mod), a `radius-m` above 0 and
at most 64, an optional `margin-m` from 0 to 64 (16 when absent), an optional
`centre` Template, a count of `plots` of at most 24, from 1 to 24
`settlement-plot` rows of a `template`, a `weight` above 0 and a `min` and `max`
count, an overall `minimum`, and a `path` Material with an optional
`path-shape`. A World holds at most 16. A `biome-settlement` row names a
`settlement` by full name and a `spacing-m` at least twice its radius and margin
and at most 4096 m; a Biome lists at most 2. A Declaration for 0.1 or 0.2 loads
with none, and one built against the earlier 0.3 text needs a rebuild. A third
row, a radius over 64 m, more than 24 Plots or an unknown Template is a load
error naming the Settlement. A World whose Biomes list none generates exactly as
before, with no generator version change.

Every Material a `template-def` names, and every `settlement-def`'s `path`, must
be Obtainable in the World: laid, grown or dropped by worldgen, or made by a
Recipe from what is. A breach refuses the World, naming the Template, the
palette character (or `foundation`) and the Material, and a Grown Material or a
Door's open Material in a `template-voxel` is refused outright. This is a loader
rule, not a contract field
([Loader rules are not the contract](#loader-rules-are-not-the-contract)).

A `biome-settlement` then gained `chance`, after `spacing-m`, above 0 and at
most 1; a TOML Declaration that leaves it out gets 1. Each lattice square draws
a candidate centre anywhere in it, which stands with `chance` unless a
neighbouring square's candidate with a higher draw is near enough for their
discs and margins to meet. A Declaration built against the earlier 0.3 text
needs a rebuild.

A `biome-settlement` then gained `near-spawn`, a `bool` after `chance`: the four
lattice squares round the origin always draw a centre, clear of the spawn's
keep-out, and win against their neighbours. A TOML Declaration that leaves it
out gets false. A Declaration built against the earlier 0.3 text needs a
rebuild.

A `settlement-def` then gained `outskirts`, an optional `settlement-outskirts`
record after `loot`: a list of `settlement-plot` rows, each Template no longer
than the Settlement's margin, and a `count-min` and `count-max` of at most 96.
Its Buildings stand beside the approach paths just past the disc, inside the
margin. Absent lays none, so a Settlement declared without it lays out as
before, and a TOML Declaration that leaves it out gets none. A Declaration built
against the earlier 0.3 text needs a rebuild.

A `settlement-outskirts` then gained `chance`, an optional `f32` last: the share
of a Settlement's copies that lay any outskirts, above 0 and at most 1. Absent
is 1, every copy. A copy that cannot stand `count-min` now lays none, so no
field stands alone. A Declaration built against the earlier 0.3 text needs a
rebuild.

A `settlement-def`'s `outskirts` then became a list of groups,
`option<list<settlement-outskirts>>`, at most 4, laid in order and each drawn by
its own chance: fields in one, ponds in another. The first group's draws are as
before; a later group never moves them. A TOML Declaration may still give one
table. A Declaration built against the earlier 0.3 text needs a rebuild.

A `biome-def` then gained `selectors`, an optional `biome-selectors` record of
`selector-range`s for `altitude`, `coast`, `erosion` and `river` and an optional
`relief`, and `strata`, an optional list of up to 8 `biome-stratum` records
(`material`, `depth-min-m`, `depth-max-m`); absent, they ask nothing.
`world-params` gained `lava`, an optional fluid name, and `cave-biomes`, an
optional list of up to 16 `cave-biome-def` records with their `cave-feature`s;
both are the Game's alone. A Declaration built against the earlier 0.3 text
needs a rebuild.

A `settlement-def`, a `settlement-plot` and a `biome-def` then each gained an
optional `loot`, last, a `loot-table` record: `rolls-min` and `rolls-max` (at
most 27) and 1 to 16 `loot-entry` rows of an `item` by full name, a `min` of at
least 1, a `max` no more than the Item's stack, and a `weight` above 0. A chest
worldgen stands in a Settlement building draws from its row's table, else the
Settlement's; one in a wilderness Structure from its Biome's. Every Item is
checked once every Mod has declared, and need not be Obtainable another way.
Tables are frozen in the generation snapshot with the Settlements and Biomes.
`none` stocks nothing, so a Declaration without loot loads as before but needs a
rebuild against the new record fields.

A `tree-def` of line 0.3 then gained `crowding-m`, an optional `f32` last: how
near, in metres, a stronger tree's trunk keeps this one from generating, above 0
and at most 12. Absent is 1.5 times `canopy-radius-m`, so a tree declared
without it crowds as before; a dense stand of one species declares less. The
Biome snapshot freezes it as an optional member, so no `FORMAT_VERSION` changes.
A Declaration built against the earlier 0.3 text needs a rebuild.

A `tree-def` of line 0.3 then took `canopy` as a list and gained `storeys`, a
list of `storey-def` after it: one canopy or up to four alike but for colour,
and up to four weighted height bands, each with a trunk height and an optional
crown. Each tree draws one of each by the World seed and its root position in
metres, and a storey's `branching` is the row's scaled by its crown over the
row's. One canopy and no storeys generate as before. The Biome snapshot freezes
both as optional members, so no `FORMAT_VERSION` changes. A data-only row's
`canopy` still takes one name; a Declaration built against the earlier 0.3 text
needs a rebuild.

A `view-params` of line 0.3 then gained `low-lod-distance-m`, an optional `f32`
last: the reach of the Low Quality curve, the Game's own rings, from
`view-distance-m` to `lod-distance-m`. `lod-distance-m` is now the ceiling every
Player's curve is clamped to, at most 8192 m. Absent, Low reaches the ceiling,
so a Game that declared its view before streams the same rings under Low. A
Declaration built against the earlier 0.3 text needs a rebuild.

An `appearance` of line 0.3 then gained `variation`, a closed `tile-variation`
last: `fixed`, `turned` (each Voxel turns or mirrors the texture, keyed on its
position in the World) or `turned-and-patched` (also shifting value and hue in
patches about 5 m across). The 0.1 and 0.2 adapters and an omitted data-only
`variation` give `fixed`, which draws exactly as before. A Declaration built
against the earlier 0.3 text needs a rebuild.

`world-params` of line 0.3 then gained `sky`, an optional `sky-params` record
last: `day-length-s` (at least 60), `start-hour` (from 0 up to 24) and
`day-share` (above 0 and below 1). It is the Game's alone, like `climate`.
`none` holds the World at noon, so a Game that declares no sky looks as before.
A Declaration built against the earlier 0.3 text needs a rebuild.

`sky-params` then gained `tilt-deg` last, from 0 to 60: how far the sun's path
leans toward the south (+Z), so the noon sun stands that far from overhead. A
Declaration built against the earlier 0.3 text needs a rebuild.

`sky-params` then gained `day-curve` last, an optional list of 1 to 16 `day-key`
records with ascending hours from 0 up to 24, each the sun's, the moon's and the
ambient light's colour and strength (`curve-light`), a sky and a fog tint
(`curve-tint`, a colour and an amount) and a star brightness. Colours are linear
and every value from 0 to 1; each channel of the ambient colour times its
strength is at least the cave floor, 0.0642. `none` is the engine's default
curve, so a Game that declared a `sky` before looks as it did. A Declaration
built against the earlier 0.3 text needs a rebuild.

`sky-params` then gained `clouds` last, an optional `cloud-layer` record: the
base's `altitude-m`, above the top of the Game's terrain band (Sea level plus
the landforms' height and overhang); `thickness-m`, from 1 to 200; `cover`, from
0 to 1; and the wind, `wind-direction-deg` (where the clouds drift toward,
clockwise from north, -Z) and `wind-speed-m-per-s`, from 0 to 50. `none` draws
no clouds. `biome-def` gained `cloud-cover` after `cave-scale`, an optional
multiplier from 0 to 4 on the Game's cover over that Biome; `none` is 1. A
Declaration built against the earlier 0.3 text needs a rebuild.

`ambient-bed` then gained `when` last, an optional closed `bed-hours` of `day`,
`night` or `always`: a day bed plays at its volume times Daylight, a night bed
times one minus Daylight, so they crossfade at dawn and dusk; `none` is
`always`. `soundscape-def` gained `night-music` last, an optional list of at
most 8 `music` Sounds by full name: when set, a track starts from `music` while
Daylight is at least 0.5 and from `night-music` otherwise, and a playing track
is never cut by the hour; `none` plays `music` at every hour, so a Soundscape
declared before sounds as it did. A Declaration built against the earlier 0.3
text needs a rebuild.

## What is not frozen

Line `0.x` is pre-release. Later gameplay steps may add new lines, and
publishing a starter does not freeze the Tools, commands or simulation features
those steps add. The final **1.0 contract freeze belongs to roadmap step 30**
(release hardening); until then, treat any new line as possible.

Saved Worlds are separate: they require the exact build that created them
([ADR-0018](adr/0018-games-and-mods-install-side-by-side.md)), whichever line it
used.

Unreleased 0.3 also appends `cause-sounds` to `declaration`, a Game-only list of
`cause-sound` records binding the closed `sound-cause` enum to Effects Sounds,
and appends optional `soundscape` after `surface-features` in `biome-def`.
Earlier lines adapt to no Cause bindings and no Biome override.

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

The unreleased 0.3 `sound-cause` enum appends `burn`: the Server emits it once
when the body enters a fluid whose declaration has `heat`, including a direct
cold-to-hot fluid transition. Bind it through Game `cause-sounds`, including in
data declarations; an unbound Cause is silent. Cold-fluid entry/exit uses
`splash`; hot-fluid exit is quiet. Hot fluids do not emit `swimming`/`floating`
or select the underwater water Soundscape. Burn uses the Vitals detail trim.
Health damage and hurt/death timing are unchanged. Older 0.1/0.2 declarations
retain their existing adaptation without Cause bindings.
