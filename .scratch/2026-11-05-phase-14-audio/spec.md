Status: ready-for-human

# Phase 14: Audio

DESIGN.md roadmap step 14. The Engine adds an audio engine, Server-driven Sound
events and mixing; a Game or Mod supplies the Sounds and music. No new
dependency.

Vocabulary: **Sound**, **Sound event**, **Cause**, **Sound set**, **Sound
category** and **Soundscape** are defined in [GLOSSARY.md](../../GLOSSARY.md).
The decision record is
[ADR-0025](../../docs/adr/0025-sounds-are-declared-data-the-server-names-and-the-client-plays.md).

## Problem Statement

Every World is silent. Breaking stone, walking on gravel, falling into a lake and
picking up an item give no feedback but pixels, and nothing tells a Player they
have gone underground, underwater or into another Biome except what they see. A Game author has no
way to give their Game a sound or a piece of music, and a Mod author cannot give
the Materials they add a voice. Settings has no Audio category, so a player could not turn any of it up or down.

## Solution

A Game or Mod declares Sounds as data: Ogg Vorbis recordings, how loud they are,
how far they carry in metres and which Sound category they belong to. Materials
get their voice through a Sound set: what breaking, placing, striking, stepping
on and landing on them sounds like. The Game binds the engine's other Causes,
such as a splash, a Pickup, eating or being hurt, to Sounds. A Soundscape
(ambient loops and music) is declared for the open world, for underground and
for underwater, and a Biome may name its own.

The Server decides when a Sound event happens and tells the Client which Sound,
where, and why; the Client chooses the recording, volume and pitch, attenuates
it by distance in metres, pans it, and plays it. Ambience and music follow the
Soundscape the Server says applies to the Eye. A player's volume for Master,
Effects, Ambient and Music is set in a new Audio category of Settings, saved in
their profile, and applies to sounds already playing. The Terra Game ships CC0 sounds as the reference; the other
demo Games stay silent, showing that sound is optional.

## User Stories

1. As a player, I want to hear a Voxel break when I remove it, so that the
   action feels physical.
2. As a player, I want to hear a Voxel being struck while I hold Remove, so that
   I know the removal is in progress.
3. As a player, I want to hear a Voxel being placed, so that I know the
   placement happened.
4. As a player, I want to hear my footsteps on the Material under my feet, so
   that grass, stone and wood feel different.
5. As a player, I want my footsteps to come faster when I sprint, so that the
   sound matches the motion.
6. As a player, I want crouching to be silent, so that sneaking feels like
   sneaking.
7. As a player, I want no footsteps while flying or swimming, so that what I
   hear matches how I move.
8. As a player, I want to hear a landing after a fall, so that falls have
   weight, without a thump for every small step down.
9. As a player, I want to hear a splash when I enter or leave water, so that the
   boundary is unmistakable.
10. As a player, I want to hear a Pickup, so that I notice an item joining my
    Inventory.
11. As a player, I want to hear whether a Use succeeded or was refused, so that
    I do not have to read the message.
12. As a Survival player, I want to hear myself eat, get hurt and die, so that
    my Vitals changing is not only a bar moving.
13. As a player, I want a sound to be louder the closer I am to it and silent
    beyond its range, so that I can judge distance by ear.
14. As a player, I want a sound on my left to be heard on my left, so that I can
    tell where it came from.
15. As a player, I want repeated sounds to vary slightly in recording, volume
    and pitch, so that they do not sound mechanical.
16. As a player, I want ambient sound around me in the open world, so that the
    World feels alive.
17. As a player, I want the ambience to change when I go underground, so that
    caves feel enclosed.
18. As a player, I want the ambience to change when my Eye goes underwater, so
    that being submerged feels different.
19. As a player, I want the ambience and music to change when I walk into a
    Biome that has its own Soundscape, so that a desert and a tundra sound
    different.
20. As a player, I want ambience to crossfade rather than cut, so that moving
    between places is not jarring.
21. As a player, I want music to play now and then with silence between tracks,
    so that it accompanies rather than overwhelms.
22. As a player, I want music never to repeat the track that just played, so
    that it feels varied.
23. As a player, I want to set Master, Effects, Ambient and Music volume
    separately in an Audio category of Settings, so that I can, for example,
    mute music and keep effects.
24. As a player, I want the Audio category to apply, discard and reset like
    every other Settings category, so that it behaves as I expect.
25. As a player, I want a volume change to apply to sounds already playing, so
    that turning music down does not wait for the next track.
26. As a player, I want my volumes remembered between launches, so that I set
    them once.
27. As a player, I want sensible default volumes, so that a new install sounds
    right before I touch anything.
28. As a player, I want a World whose Game declares no sound to be silent, not
    broken, so that sound is never required.
29. As a Game author, I want to declare a Sound with one or more weighted
    recordings, so that common variants play more often.
30. As a Game author, I want to declare a volume range and a pitch range per
    Sound, so that I control how much it varies.
31. As a Game author, I want to declare a Sound's reference and maximum distance
    in metres, so that it carries the same distance at any Voxel size.
32. As a Game author, I want to put each Sound in a Sound category, so that the
    player's volume settings apply to it correctly.
33. As a Game author, I want to declare a Sound set once and give it to many
    Materials, so that every stone-like Material shares one voice.
34. As a Game author, I want a Sound set to leave out Causes it has no sound
    for, so that I declare only what I have.
35. As a Game author, I want to bind splash, Pickup, Use outcomes, eating,
    hurt and death to Sounds,
    so that the engine's non-Voxel Causes are heard in my Game.
36. As a Game author, I want to declare a Soundscape for the open world,
    underground and underwater, each with ambient loops and a music list, so
    that each place has its own atmosphere.
37. As a Game or Mod author, I want a Biome to name a Soundscape, so that the
    Biomes I add sound as different as they look.
38. As a Game author, I want to set the silence between music tracks as a
    range, so that music comes as often as my Game wants.
39. As a Game author, I want to reference recordings as files inside my Game's
    folder from a data Declaration, so that I need no code.
40. As a Mod author writing a component, I want the same Sounds, Sound sets and
    bindings through WIT, so that both Declaration formats have the same power.
41. As a Mod author, I want to add Sounds and Sound sets and give them to my own
    Materials, so that my content Mod is heard.
42. As a Mod author, I want to give a Sound set to another Mod's Material by its
    full name, so that a sound-only Mod can voice a Game.
43. As a Mod author, I want a clear load error when two Mods give the same
    Material a Sound set, so that the conflict is not resolved silently.
44. As a Mod author, I want a clear load error naming my Mod and the field for a
    missing, escaping, oversized or malformed recording, so that I can fix it
    before a World opens.
45. As a Mod author, I want a load error when a positional Sound's recording is
    stereo, so that I learn it cannot be placed in space.
46. As a Mod author, I want `tesserae validate` and `tesserae package` to check
    and carry my recordings, so that what I ship is what was checked.
47. As a Mod author, I want a changed recording to need Trust again, like
    changed artwork, so that a World's build identity covers what it plays.
48. As a Mod author, I want an older-line component to keep loading with no
    sound, so that nothing I built breaks.
49. As an Engine maintainer, I want the Server to send a Sound's id rather than
    a file, so that a future transport stays small.
50. As an Engine maintainer, I want the Server to drop Sound events beyond their
    range, so that a future transport never carries inaudible sounds.
51. As an Engine maintainer, I want attenuation, panning and music scheduling as
    pure functions in `tesserae-core`, so that they are tested headlessly.
52. As an Engine maintainer, I want no audio crate added, so that the dependency
    tree and build time do not grow.

## Implementation Decisions

### Backend

- Bevy's built-in `bevy_audio`, already on through Bevy's default `audio`
  feature (rodio and the Vorbis decoder). No crate is added, and
  `bevy_kira_audio` and `bevy_seedling` are rejected for now (ADR-0025).
- Attenuation is computed by the engine, not by rodio. Rodio's spatial source
  applies a fixed inverse-square curve with no range, so it cannot express a
  Sound's distances in metres. The spike (ticket 01) settled it: the Client plays through a plain sink with the engine's gain as its volume and the engine's own equal-power pan, from a source that wraps Bevy's decoder and downmixes to mono. Rodio's spatial sink is not used.
- Bevy's `GlobalVolume` is read only when a sound starts, so it stays at 1. The
  Client sets each sink's volume itself and updates it when a setting changes.

### The Declaration (contract line 0.3, amended in place)

- Line 0.3 is unreleased, so it is amended in place, as it was for `items`;
  no new line. The `declaration` record gains five lists, appended after
  `survival`, and `biome-def` gains an optional `soundscape`, appended after
  `surface-features`. `material-def` and `world-params` are untouched: a
  separate list matches how `appearance` attaches colour to a Material.
- `contract-compatibility.md` gains a paragraph under "Amended before release",
  in the form the Biome and Survival paragraphs use; a Declaration on 0.1 or 0.2
  loads with no sounds. The starter's WIT copy is
  updated to match.
- **`sounds`**, from any Mod: name (local, the engine adds `<mod id>:`), one or
  more recordings each with a weight, a volume range, a pitch range, a Sound
  category, and for Effects a reference distance and a maximum distance in
  metres.
  - A component supplies each recording's bytes; a data Declaration names a
    relative path inside the Mod folder, read with the same path rules as PNG
    artwork.
  - Sound category is a closed enum: `effects`, `ambient`, `music`. Effects are
    positional and must be mono; Ambient and Music are not positional and may be
    stereo. Distances are required for Effects and refused for the others.
- **`sound-sets`**, from any Mod: name (local) and an optional Sound for each
  Voxel Cause: `break`, `place`, `hit`, `step`, `land`. Sound names are full
  `<mod id>:<name>` names.
- **`material-sounds`**, from any Mod: a Material's full name and a Sound set's
  full name. A Material has at most one Sound set in a World; a second row for
  the same Material, from any Mod, fails the load naming both Mods.
- **`cause-sounds`**, Game only: a non-Voxel Cause and a Sound. The Cause enum
  is closed: `splash`, `pickup`, `use-measured`, `use-refused`, `eat`, `hurt`,
  `death`. A new Cause
  needs new engine mechanism and so arrives with a contract change.
- **`soundscapes`**, from any Mod: name (local), an optional context, a list of
  ambient beds (an Ambient Sound and a volume), a list of Music Sounds, and the
  silence between tracks as a range in seconds, defaulting to 60 to 180. The
  context enum is closed: `default`, `underground`, `underwater`. Only the
  Game's Soundscapes may carry a context, and at most one per context; a Mod's
  Soundscape exists to be named by a Biome.
- **`biome-def.soundscape`**: an optional Soundscape by full name, from any Mod
  that declares Biomes.
- Every name in these lists resolves after every Mod has declared, in
  dependency order, like fluids and reactions. An unknown name, a Sound of the
  wrong category in a slot (a Music Sound as a bed, an Ambient Sound in a Sound
  set), or a value out of range fails the load naming the Mod and the field.
- `cause-sounds`, or a Soundscape with a context, from a Mod that is not the
  Game is a load error, like `tools`.

### Recordings and limits

- Ogg Vorbis only. The loader reads the first Ogg page and the Vorbis
  identification header by hand to check the format, the channel count (1 or
  2) and a sample rate from 8 to 96 kHz. `tesserae-mods` gains no decoder.
- Limits, validated per Mod and per World: at most 512 KiB per Effects or
  Ambient recording, 8 MiB per Music recording, 8 recordings per Sound, 64
  Sounds and 16 Sound sets per Mod, 8 Music Sounds per Soundscape, and 32 MiB
  of recordings per World.
- Ranges: weight 1 to 16; volume 0 to 1; pitch 0.5 to 2; reference distance
  0.25 to 16 m; maximum distance above the reference and at most 128 m.
- Referenced recordings join build identity with the Manifest, the Declaration
  and artwork. Changing one needs Trust again; unreferenced audio files do not.
  `tesserae package` copies referenced recordings at their relative paths.
- Recordings reach the Client as in-memory audio sources made from the checked
  bytes, not through the asset server. Music is held in memory for the World
  session; that cost is accepted.

### Server

- A **Sound event** is a plain-data message: the Sound's id, a position in
  metres, and its Cause. It is the shape a future transport carries; it never
  names a file.
- The Server drops a Sound event whose position is beyond its Sound's maximum
  distance from the listening Player's Eye. Ambient and Music Sounds are not
  Sound events.
- Voxel Causes, using the Sound set of the Material involved:
  - `break` when a Player's deliberate Remove takes a solid out of the World,
    from the same point that drops an Item. Undo, redo, Layout, fluid, fire and
    a Mod's Commands make no Sound event.
  - `place` when a Player's single Place lands a solid or a source; the
    source's Sound set is its fluid Material's. Layout makes one `place` at its
    anchor, not one per cell.
  - `hit` while a Player holds Remove on a target: the aim report carries a
    flag that Remove is held, and the Server emits `hit` every 0.25 s while the
    target is unchanged and in reach. Removal progress stays the Client's; the
    flag grants nothing.
  - `step` every 1.5 m of horizontal travel while grounded and not crouching,
    from the solid under the feet. Flight and swimming make no steps. Sprinting
    keeps the stride, so steps come faster.
  - `land` when the body becomes grounded after falling 1 m or more, from the
    solid landed on.
- Other Causes, using the Game's `cause-sounds`: `splash` when the body enters
  or leaves a fluid; `pickup` on each Pickup; `use-measured` and `use-refused`
  on each Use outcome, eating included; `eat` when food is eaten; `hurt` on each
  damage a Player takes, whatever its cause; `death` when a Player dies.
- Stride, fall height and hit interval are engine constants in metres and
  seconds. An unbound Cause, or a Material with no Sound set, is silent.
- **Soundscape**: each tick the Server works out the Eye's context and sends
  the Soundscape's id when it changes. Priority: underwater (the Eye is inside a
  fluid), then underground (the Eye's Voxel receives no sky light), then the
  Soundscape the Eye's Biome names, then default. A context with no Soundscape falls through to the next; none at all
  is silence. A context is a light level and an immersion test, never a depth
  in metres or Voxels.

### Client

- The listener is the Eye in both view modes, matching how reach and sight are
  judged; panning follows the camera's facing.
- For each Sound event the Client asks the core audio module for a playback
  plan and starts a sink with it. It never decides that a Sound event happens.
- Ambience: each bed of the current Soundscape loops; on a Soundscape change the
  old beds fade out and the new fade in over 2 s.
- Music: after a silence drawn from the Soundscape's range, a random track that
  is not the one that just played. On a Soundscape change the current track
  fades out over 4 s and the new Soundscape's silence begins.
- Volume settings live in the user profile beside view mode: Master, Effects,
  Ambient and Music, from 0 to 100, defaulting to 100, 100, 70 and 50. Every
  playing sink's volume is recomputed when one changes.
- Settings gains an Audio category, which the menus and Inventory work (#57)
  left hidden until this step: four sliders from 0 to 100, through the same
  draft, Apply, Discard and Reset as every other category. Its place follows
  the prototype: Behavior, Key bindings, Controller, Display, Audio,
  Accessibility. The mockup is that work's prototype Audio page (Master, Music,
  Effects) with an Ambient row, agreed with the user before implementation.

### Core audio module (no Bevy)

- Pure functions and a state machine in metres and seconds:
  - attenuation: inverse-clamped, 1 within the reference distance, falling as
    reference over distance beyond it and reaching 0 at the maximum distance;
  - pan: from the emitter's direction relative to the listener's facing;
  - variation: recording by weight, volume and pitch within their ranges, from
    a seeded random source so tests are deterministic;
  - gain: Master × category × Sound volume × attenuation, each setting mapped
    from 0–100;
  - music scheduler: given elapsed time, the current Soundscape and its tracks,
    what is playing, fading or waiting.
- The Bevy glue in the Client only turns a plan into a sink and keeps sink
  volumes in step with settings.

### Content and docs

- Terra gains Sound sets for its Materials, every Cause binding, Soundscapes for
  the three contexts, and a Soundscape for each of its Biomes that should sound
  different. Recordings are CC0 only (Kenney,
  OpenGameArt or freesound entries that are CC0), converted to Ogg Vorbis
  locally, within the 32 MiB referenced World budget. Longer beds and full music
  tracks replace the initial 2 MiB target. Each recording is listed in a `CREDITS.md` in
  Terra's folder with its source URL and licence. No CC-BY, no non-commercial
  licence, no Sonniss bundle.
- The data-only Mod guide, the authoring guide's schema, `architecture.md`,
  `contract-compatibility.md` and the DESIGN.md roadmap row are updated.

## Testing Decisions

- A good test drives external behaviour at the highest seam: given a
  Declaration and a World, which Sound events and Soundscape changes come out,
  or given a Sound event and a listener, what plays. No test asserts on private
  fields.
- **Seam 1: Declaration loading** in `tesserae-mods`: valid Sounds, Sound sets,
  `material-sounds`, `cause-sounds` and Soundscapes in both formats; every
  refusal names the Mod and the field (malformed Ogg, stereo Effects,
  distances on Music, limits, unknown names, wrong category in a slot, two sets
  for one Material, Game-only lists from a Mod, an escaping path); older lines
  adapt to no sounds; recordings join build identity and `package` output.
  Prior art: `tests/declare.rs` and `tests/loading.rs`. Fixtures are tiny Ogg
  files generated once and committed, like the PNG fixtures.
- **Seam 2: the headless Server app**, driving real messages: Remove, Place and
  a held hit produce Sound events with the right Sound and position; undo,
  Layout cells and Commands do not; steps per 1.5 m, none crouching or flying;
  `land` only after a fall of 1 m or more; splash, Pickup, Use outcomes,
  eating, hurt and death; events beyond range dropped; Soundscape sent only on
  change and in priority order, Biomes included. Prior art: the `server::edit`,
  `server::tool`, `server::entities` and `server::vitals` tests.
- **Seam 3: the core audio module**, headless: attenuation at and beyond each
  distance and at different Voxel sizes; pan left and right; seeded variation
  stays in range and honours weights; gain from settings; music scheduler
  gaps, no repeat, fades on change. Prior art: `collide` and `immersion` tests.
- **Seam 4: the user profile and Settings drafts**: the four volumes round-trip
  and default to 100, 100, 70 and 50; the Audio category's drafts apply only on
  Apply and Discard and Reset restore them. Prior art: the `client::profile`
  and `client::settings` tests.
- No test touches an audio device. The spike ticket is checked by ear; the
  finished Terra sound is listened to once by the user.

## Out of Scope

- A `night` context. The engine has no time of day yet (step 9's cycle); the
  context enum gains it with that work.
- Positional ambient emitters (a waterfall, a fire's crackle), and any sound
  for fire, fluid flow, reactions or combustion.
- Occlusion, low-pass filtering, reverb, ducking, HRTF and Doppler.
- Sounds for mobs, NPCs and other Entities beyond Dropped items (steps 22–23).
- A Command or host function for a Mod to play a Sound. Recorded as a contract
  gap for the step that needs it.
- UI sounds such as menu clicks.
- Per-fluid splash Sounds; splash is one Game binding.
- Client-side prediction of Sound events; an in-process Server has no
  latency worth hiding.
- Formats other than Ogg Vorbis.
- Network transport.

## Further Notes

- Prior art: Minecraft's `sounds.json` (an event table of weighted variants with
  volume, pitch and range) and block sound types; Luanti's node `sounds` and
  numbered variant groups (Ogg only, mono for positional); Vintage Story's block
  `sounds` keys. Minecraft moved biome ambience and music out of biome effects
  in 1.21.11, so its old biome schema is not a model.
- Rodio 0.22's spatial source attenuates by 1 / distance² clamped at 1, with
  panning from the distance to each ear. That is the measured reason
  attenuation is the engine's.
- Rebased onto main after Biomes (#56) and menus, Inventory and the Survival
  loop (#57) merged. Both appended to the 0.3 `declaration` and took ADR-0022 to
  ADR-0024, so this step's record is ADR-0025. Carried is retired: Pickup fills
  the Inventory.

## Follow-up: Biome recordings and water movement

Terra's temperate, dry and cold Biomes now use distinct Ambient recordings:
birds/breeze, band-limited dry wind, and lower cold gusts. Dry and cold are
separately processed variants of a credited CC0 wind loop. Music remains shared.
The Server also emits Game-bound `swimming` and `floating` Causes: strokes follow
1.2 m of movement, and floating water sounds repeat at seeded 6–12 s intervals while stationary
at the surface. Flying, grounded wading, death and paused play emit neither.
See ticket 12 for validation and the updated authoring guides for bindings.

## Approved audio comfort revision

The user approved the detailed Audio settings mockup in ticket 13. The engine
now persists ten per-action trims and applies them to already-playing effects.
Terra uses long seam-edited beds and complete independent music tracks, with
quiet, varied floating details and smooth ambience ducking during audible
music. Old profiles retain neutral trims. The 32 MiB referenced World budget
remains; Ambient has its own 4 MiB file ceiling. See
[validation](comfort-validation.md) for measurements and the listening checks
that remain open. No new dependencies, commit or push.

## Listening feedback: green-area tone and menu noise

The user confirmed the “ooo” remains with Music muted and occurs in green
areas. Replace the temperate bird-and-synth bed with neutral field-recorded
wind/rustle; remove the bird-containing music track from selection. Future bird
calls belong to nearby animal entities and positional Server events, not global
Biome beds. No animals are added by this change.

Menu, Settings and save modals fade World playback down over 0.1 s in real time,
pause background sinks and music scheduling, and discard one-shots rather than
replaying stale actions. Resume fades audio back in without restarting the
track. Music volume changes go through Apply and become audible after resuming;
Ambient independently controls environmental beds. The Music-slider regression
covers draft change through Apply to the existing track, at 0 and 100.

## Listening feedback: sustained lava damage

User found the lava damage/death sound and feedback animation too rapid and
short. Health loss remains continuous and unchanged, but repeated `hurt` audio
is limited to one event per 0.75 s. Death remains a single transition event;
dead Players do not emit more damage feedback, and the next life starts fresh.
The Client injury flash lasts 0.65 s without restarting on every decrement;
low-health edge pulses take 3 s and health tile pulses 1.5 s. Reduced motion
keeps its existing still presentation. Terra's death cue has a single roughly
1.56 s release at fixed pitch. These are feedback timings, not invulnerability
frames or a change to how fast lava kills a Player.

## Listening feedback: surface movement

Forward motion while floating uses Swimming feedback. The distance-only
1.2 m trigger overlapped short splash clips at higher speeds. It now requires
roughly 3 m of travel and a seeded 1.6–2.2 s cooldown; accumulating extra travel
cannot queue bursts, and brief stops or water exit/re-entry cannot reset the
cooldown. Terra supplies three quiet 1.2 s water gestures; even at minimum
pitch they end before the next stroke. Idle Floating retains its 6–12 s gaps.
Fluid physics and actual swimming speed are unchanged.

## Listening feedback: footsteps and stone removal

Walking must use real footstep recordings rather than clones of material break
sounds. Terra uses varied CC0 stone and grass/sand steps, and quiet generic shoe
steps for wood and metal. Stone removal uses three actual rock-breaking takes
instead of a door closing. Shape sharp attacks and tails and keep the walking
cadence, Server events, controls and movement unchanged. Existing water feedback
is outside this asset correction. Listening remains a human acceptance check.

## Startup crash: fractional coarse water

A reported async Chunk palette panic exposed a coarse-water rendering hazard:
looking up the cell under a partial-height surface truncates the height before
subtracting one. A bottom-row surface underflows; higher fractional surfaces
read the preceding row. Round the top upward before subtracting one, preserving
exact integer tops and the distinction between generated lakes and moved fluid.
Regressions must check bottom-row partial water, every row and full surfaces,
and preserve flowed-water geometry. This correction changes no audio behavior.

## Listening feedback: falling into water

Use a real water-impact recording for the Server's Splash event, with a clear
initial splash and settling water tail. Give it its own Terra sound rather than
sharing the water Material's short interaction sample. Keep swimming, floating,
water footsteps, event timing and settings unchanged. Use CC0 field-recorded
water, mono 44.1 kHz Vorbis, without pitch alteration. Listening acceptance
remains open until the user tries falling into water in game.

## Lava is not water

Fluid entry must distinguish declared heat-emitting fluids from cold fluids.
The Server emits a new `burn` Cause once on hot-fluid entry, including a direct
water-to-lava transition, rather than Splash. Hot-fluid exit is quiet; cold
fluid entry/exit keeps Splash. Initial spawn in fluid keeps the existing silent
initialization. Hot fluids do not produce Swimming/Floating or select underwater
water ambience. Terra binds `burn` to a credited CC0 fire recording at natural
pitch and uses its burning material set for lava contacts. Health damage and
hurt/death pacing remain unchanged. Append `burn` to unreleased 0.3 only; older
contract lines adapt without it, and an unbound burn Cause stays silent.
