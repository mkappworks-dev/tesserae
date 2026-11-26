# Day and night cycle

Status: ready-for-agent

Source: branch `feat/day-night-cycle`, cut from `main` at `0b03c594`. Builds on
the colour spec, `.scratch/2026-11-23-colour-art-and-shader/spec.md` on branch
`fix/colour-art-and-shader` (not yet on `main`), which lands the gradient sky,
the sun disc and the light balance this spec moves. Tickets: [issues/](issues/).

## Problem Statement

The sky never changes. The sun stands at one fixed angle, the light is noon
forever, there is no moon and no stars, and no clouds. A World feels like a
still photograph: nothing tells a Player that time passes, and the Soundscape
plays the same birds at every hour. DESIGN.md's step 9 promised "a day/night
cycle" as part of the sky's polish and never delivered it.

## Solution

A World has a **Time of day** the Server owns and saves. The sun rises in the
east, crosses the sky on a path tilted toward the south and sets in the west;
the moon follows the opposite path and shows one of eight **Moon phases**, a new
one each night. Light, the ambient dome, the sky and the fog follow a **Day
curve** the Game declares: warm at dawn and dusk, blue and dim at night, with
stars fading in and turning with the sky. Torches stand out at night because
Block light does not change, and caves look the same at every hour. A **Cloud
layer** drifts across the sky at an altitude in metres, coloured by the same
light. Soundscapes blend day and night beds, so birds give way to crickets at
dusk. A World can switch its **Daylight cycle** off, which holds the sky still.

Terra declares a 20-minute day. A World with no Game, or a Game that declares no
sky, keeps today's fixed noon.

Weather (step 24) later adds itself to the **Sky state** as one more layer
(cloud cover, darkening, rain), without rewiring anything this spec builds.

## User Stories

1. As a Player, I want the sun to rise and set, so that the World feels alive
   and time visibly passes.
2. As a Player, I want shadows to follow the sun, so that the light reads as
   coming from it.
3. As a Player, I want noon shadows that are not straight down, so that shapes
   still read at midday.
4. As a Player, I want dawn and dusk to tint the sky, the fog and the light
   warm, so that the change from day to night is gradual and beautiful.
5. As a Player, I want night to be dark and blue but readable, so that I can
   still find my way without a torch.
6. As a Player, I want a full moon to light the night more than a new moon, so
   that the nights differ and the moon matters.
7. As a Player, I want to see the moon's phase on its disc, so that I can tell
   which night of the cycle it is.
8. As a Player, I want stars to fade in at dusk and turn with the sky, so that
   the night sky is worth looking at.
9. As a Player, I want my torches and lanterns to stand out at night, so that
   light I place matters.
10. As a Player, I want a cave to look the same at noon and at midnight, so that
    going underground is not affected by the hour.
11. As a Player, I want water to darken at night, so that a night dive does not
    show daylit blue.
12. As a Player, I want clouds that drift across the sky, so that the sky moves
    even when I stand still.
13. As a Player, I want clouds lit by the sun or the moon, warm at dusk and grey
    at night, so that they belong to the same sky.
14. As a Player flying high, I want to fly up through the clouds and see them
    from above, so that they are a place and not a painting.
15. As a Player, I want a dry Biome to have fewer clouds than a swamp, so that
    each region's sky has character.
16. As a Player, I want the clouds to fade into the distance like the terrain,
    so that they do not end in an edge.
17. As a Player, I want birds by day and crickets by night, so that I can hear
    the time.
18. As a Player, I want day and night sounds to blend at dawn and dusk, so that
    the change is never a cut.
19. As a Player, I want music suited to the night after dark, so that the mood
    follows the hour.
20. As a Player, I want the Menu's "You are here" to show the day and the hour,
    so that I can plan around the night.
21. As a Player, I want a World I reopen to continue at the hour I left it, so
    that saving does not reset the day.
22. As a Player, I want to turn the Daylight cycle off when creating a World, so
    that a building World can stay at the hour I like.
23. As a Player, I want to turn the Daylight cycle on or off for an existing
    World in Edit World, so that I can change my mind.
24. As a Player, I want the sky to be the same at every Voxel size, so that a
    World with smaller Voxels looks the same.
25. As a Player, I want the map to stay daylit, so that it stays readable at
    night.
26. As a Player, I want the day and night to cost no noticeable frame time, so
    that the game stays smooth.
27. As a Game author, I want to declare how long my day is, how much of it the
    sun is up and the hour a new World starts at, so that my Game sets its own
    pace.
28. As a Game author, I want to declare a Day curve of light and colour
    keyframes, so that my Game's dawn, noon, dusk and night look the way I want.
29. As a Game author, I want my Biomes' sky and fog colours to stay the daytime
    colours the Day curve tints, so that I declare each Biome once.
30. As a Game author, I want to declare the sun's tilt, so that my Game's sky
    has its own character.
31. As a Game author, I want to declare the clouds' altitude, thickness, cover
    and wind, and let a Biome scale the cover, so that my Game's sky has its own
    look before Weather exists.
32. As a Game or Mod author, I want to mark a Soundscape bed as day-only or
    night-only, and give a Soundscape night music, so that my sounds follow the
    hour.
33. As a Game author whose Game declares no sky, I want my Game to look and
    sound exactly as it did, so that the change costs me nothing.
34. As a Mod author, I want a declared sky to be validated with a clear load
    error, so that a typo does not give a black sky.
35. As a developer, I want to set the Time of day for a capture, so that looks
    are compared at the same hour and Moon phase.
36. As a developer, I want the Sky state computed in one pure function, so that
    every look decision is testable without a GPU.
37. As the Weather step, I want the Sky state to be one value computed from its
    inputs, so that Weather adds an input instead of rewiring the sky.
38. As a future multiplayer Client, I want the Time of day to arrive as a time
    and a rate, so that every Player sees the same sky with few messages.

## Implementation Decisions

### Time of day

- The Server owns the Time of day: a count of logical steps since day 0, so
  whole days and the step within the current one are exact integers. It
  advances one step per logical step of the simulation while the Daylight cycle
  is on ([ADR-0014](../../docs/adr/0014-simulation-advances-in-bounded-logical-steps.md)):
  no second scheduler, and under load it lags real time as the rest of the
  World does. The Client never sets it
  ([ADR-0002](../../docs/adr/0002-client-server-from-day-one.md)).
- The World is never paused: the Menu is Client playback policy and does not
  pause the simulation (`docs/architecture.md`), so the sky keeps moving behind
  it, as fluids do.
- It is saved with the World as World state. Nothing is released, so there is
  no migration: a World saved without it opens at the Game's start hour.
- The clock reads in hours from 00:00 to 24:00, fixed to the sun: sunrise is
  always 06:00, noon 12:00, sunset 18:00 and midnight 00:00. A Game's day share
  (how much of the Day length the sun is up) sets how fast hours pass by day and
  by night, so at 0.7 a night hour passes faster than a day hour. The sun's
  angle is uniform in hours. The Menu shows the first day as "Day 1".
- **Daylight** comes from the sun's elevation: 0 below -10°, 1 above +10°,
  smoothstep between. It is defined once, beside the clock, and everything that
  needs it (the light's fade, star brightness, the sound mix) reads that one
  function.
- The Moon phase is the day count of the night: it changes at noon, when the
  moon is down, so it never steps while the moon is up. Night 0 is a full moon
  and night 4 new.
- The Server sends the Client the Time of day and its rate (steps per second, 0
  while held) when the World opens, when the rate changes, when the time is set,
  and every 5 s. The Client advances it locally and eases to each correction
  over about 2 s; a correction of more than a quarter day snaps. The Client
  computes everything else from it.
- Under `dev` and `profiling` builds the Server reads `TESSERAE_TIME_OF_DAY`, an
  hour and an optional night (`"19:30"`, `"4 00:00"`, the new-moon midnight),
  and holds the clock there, for captures.
- Mods get no call to read or set the Time of day in this spec. Setting it is a
  command (step 29); sleeping and Weather add a WIT call when they need one.

### Daylight cycle

- A World setting, on by default, saved with the World; a World saved without
  it opens with the cycle on (its default is `true`, not the type's).
- New World and Edit World each get a toggle beside Game mode, mocked up in
  `mockup/` and approved first. Edit World never writes World state: as for Game
  mode, the toggle is a pending change the Server applies when the World next
  opens, worded as such in the mockup.
- Create records in the World's sidecar whether its Game declares a `sky`, so
  Edit World can tell without reading Declarations. A Game that declares no
  `sky` has no cycle: the World is held at noon and neither screen shows the
  toggle.

### What a Game declares

All in the unreleased 0.3 contract, amended in place
([ADR-0012](../../docs/adr/0012-mods-declare-once-and-command-in-batches.md)),
with the data-only TOML, the Rust starter's copy of the contract and the
contract compatibility doc changed alongside. A Mod that sets none of them
declares what it does today. Each field has one ticket that adds it.

- `world-params` gains `sky`, which only the Game may set, like `climate`:
  - the Day length in seconds, the start hour and the day share;
  - the sun's tilt in degrees: the path leans toward the south (+Z), so the noon
    sun stands that far from overhead;
  - the Day curve: keyframes at hours, each giving the sun's colour and
    strength, the moon's colour and strength, the ambient light's colour and
    strength, a sky tint and a fog tint (each a colour and an amount) and star
    brightness. Strengths are shares of the engine's noon; colours linear. Keys
    are eased linearly and wrap at midnight. A `sky` without a curve uses the
    engine's default curve;
  - the Cloud layer: altitude and thickness in metres, cover, and a wind
    direction (degrees clockwise from north, -Z) and speed in metres per second.
    `none` draws no clouds.
- `biome-def` gains a cloud cover scale; `none` is 1. The cover the sky shows is
  the Eye's Biome's scale times the Game's cover, clamped to 1.
- `ambient-bed` gains `when`: day, night or always (the default).
- `soundscape-def` gains `night-music`. When set, `music` plays by day and
  `night-music` by night; when `none`, `music` plays at every hour.
- Validation, each a load error naming the field:
  - a Day length of at least 60 s, a start hour in [0, 24), a day share in
    (0, 1), a tilt in [0, 60];
  - at least one and at most 16 keys, hours ascending in [0, 24), strengths,
    amounts and colours in [0, 1], an ambient strength at every key at least the
    cave floor's share of noon;
  - a cover in [0, 1], a cover scale in [0, 4], a cloud base above the Game's
    terrain band, a thickness of 1 to 200 m, a wind speed of at most 50 m/s;
  - `night-music` at most 8 Sounds, like `music`.

### Sky state

- One pure function in `tesserae-core`, with no Bevy (architectural invariant
  3): the Time of day, the Game's `sky`, the eased Biome pair, whether that pair
  is a Cave Biome's, and the cloud cover in; sun and moon directions, which of
  them the light is, the light's colour and strength, the ambient colour and
  strength, the sky and fog colours, star brightness and rotation, the Moon
  phase, Daylight, and the cloud cover and drift out.
- A tint is a colour and an amount: the result is the Biome colour moved toward
  the tint colour by the amount, in linear light. A multiply would turn a blue
  sky grey at dusk instead of warm. A Cave Biome's pair is returned untinted.
- The Server already sends the Biome pair or, underground, the Cave Biome's air
  in the same message; it gains a flag saying which, and the Eye's Biome cloud
  cover scale, blended at Biome borders like the pair.
- The Client keeps the eased pair in its own resource and evaluates the Sky
  state from it every frame; the tinted result goes to `FogSettings` and the
  clear colour. Feeding tinted colours back into the ease would compound the
  tint. The Client computes presentation only and writes no World state.
- Weather will be one more input, applied after the Day curve: a cover, a
  darkening and a desaturation.

### Light

- One shadowed `DirectionalLight` is the sun while the sun is up and the moon
  while it is down. The engine multiplies its strength by a horizon fade (the
  sun's or moon's own Daylight), whatever the curve says, so it is zero at the
  horizon, where its direction swaps, and the swap never shows. There is still
  one shadow caster. It moves every frame; stepping it in 0.25° steps is the
  fallback if shadow edges crawl on a capture.
- Moonlight is the curve's moon colour and strength times how full the Moon
  phase is. Terra's night at full moon is about 15 to 20% of noon on open
  ground, blue; at new moon about 8%.
- Night is uniforms only: the light, the ambient dome, the sky and fog colours.
  Sky light is how much of the sky a face sees, so nothing is relit or remeshed,
  at any LOD level.
- The dim floor a face that sees no sky keeps is the noon floor, carried in its
  own uniform, not scaled by the hour, so a sealed cave looks the same at every
  hour. Validation keeps the curve's ambient at or above that floor, so open
  ground is never darker than a cave. Block light is unchanged.
- Underwater fog and the clear colour behind it dim and tint with the ambient
  light, so water is dark at night; a fluid that emits (lava) keeps its own.
  Cave fog does not change with the hour.
- `DistanceFog` gains a glow toward the light (`directional_light_color`, new;
  none is set today) from the light's colour, so a low sun warms the fog toward
  it and it fades at night.
- The map stays daylit: it draws captured colours, not the sky.

### Sky

- Drawn by our own shader, not Bevy's `Atmosphere`, on a camera with no HDR
  ([ADR-0039](../../docs/adr/0039-the-sky-is-our-own-shader-not-bevys-atmosphere.md)).
  It extends the colour spec's gradient (a Biome's sky colour at the zenith, its
  fog colour at the horizon), with the Sky state's tinted colours, so the fog
  still ends in the colour that comes out as the sky and the reach's edge stays
  hidden.
- The sky reads the sun's and moon's directions from the Sky state, never from
  the `DirectionalLight`, which is the moon at night. The sun disc stands at the
  sun's direction and the moon disc, showing its phase, at the moon's; neither
  is drawn below the horizon.
- Stars are procedural from the view direction, rotate with the Time of day and
  fade in with the curve's star brightness. Clouds hide them.

### Cloud layer

- A translucent slab of the Game's thickness at its altitude that follows the
  Eye horizontally, out to the reach, drawn in the blended pass and casting no
  shadow. It is depth-tested like terrain and Distance fog fades it, so it is
  right from below, from above, against a peak and at the reach. Inside its
  thickness the view fogs toward the cloud colour by the cover at the Eye, so
  crossing it is a fade, not a flip.
- Shaded from 2D noise in metres, thresholded by the cover, with a few stacked
  samples for a sense of thickness. The noise is offset by the wind times the
  time since day 0, wrapped on the CPU to the noise's period so the offset
  never loses precision; it is the same for every Player and never sent.
- Lit by the Sky state's light and ambient colour.
- In metres throughout, so the same at every Voxel size
  ([ADR-0003](../../docs/adr/0003-voxel-size-is-per-world.md)).
- The terrain band's top becomes a public helper of the generator, so the cloud
  base check and the generator agree.

### Sound

- The Server keeps naming the Soundscape; the hour only changes the mix.
- The Client scales each bed by its `when`: day beds by Daylight, night beds by
  one minus it, always beds by 1, so dawn and dusk crossfade.
- A track starts from `music` when Daylight is at least 0.5 and from
  `night-music` otherwise; a playing track is never cut by the hour.
- Terra's outdoor Soundscapes (default, Temperate, Dry, Savanna, Cold) each mark
  a day bed and a night bed; Biomes may share a pair. Sounds are CC0, under the
  same licence checks as Terra's other sounds.

### Terra

- A 20-minute day, a day share of 0.7, the start hour 07:00, a tilt of 25°.
  With Daylight's ±10° band that is about 12 minutes of full day, 1 minute each
  of dawn and dusk, and about 5 minutes of full night.
- A Day curve whose values the user approves on captures.
- Clouds based at 300 m, above its terrain band's top (sea level 24 m plus 192 m
  of landform height and 12 m of overhang, 228 m), 30 m thick, cover 0.4, a slow
  wind; Dry and Savanna scale the cover down and Swamp up.

### Order and other sessions

- Tickets 04 to 07 build on the sun disc and rebalanced light that
  `fix/colour-art-and-shader` merged to `main` (#75).
- Tickets that amend `sky` run in order (01, 04, 05, 07), so they never edit the
  same record at once.
- The Look overhaul (`feat/look-overhaul`) has been told: nothing here needs
  HDR; the sky, clouds and Day curve are display-referred; if it turns HDR on,
  its exposure must keep the fog colour equal to the colour that comes out as
  the sky; a sun disc that blooms is its choice. Under HDR it plans to
  pre-compensate for the tonemapper so authored sky, fog and Day curve colours
  stay what is seen; if it decides otherwise instead, Terra's Day curve is
  re-picked on captures.
- ADR numbers: `feat/water-look` holds an uncommitted ADR-0038, so this spec's
  is 0039. Whichever branch merges second renumbers on a clash.

## Testing Decisions

A good test drives the behaviour from the outside: a Time of day and a declared
`sky` in, a Sky state, a light, a fog colour or a bed volume out. It never
asserts on a shader's internals or a private field. Demo Game tests are on Terra
only.

- **The Sky state function** is the main seam and carries most cases:
  - sunrise is at 06:00 and sunset at 18:00 under any day share, and the noon
    sun stands the tilt from overhead, toward the south;
  - the light's strength is zero at the horizon on both sides of the swap,
    whatever the curve, and its direction is the moon's while the sun is down;
  - moonlight at full moon is greater than at new moon, and the phase changes at
    noon;
  - a key with amount 0 returns a Biome pair unchanged; a Cave Biome's pair is
    never tinted;
  - no `sky` gives today's noon exactly.
- **The clock**: advancing by logical steps, holding when the cycle is off,
  wrapping a day, Daylight's band, and a save and reopen at the same hour, at
  the prior art of the persistence round-trip tests.
- **The Server**: the Time of day message is sent on open, on a rate change and
  every 5 s, at the prior art of the sky and fog publishing tests.
- **The Client**: easing to a correction, snapping past a quarter day; the
  light, the ambient brightness, the fog and the clear colour follow the Sky
  state; the eased pair is never tinted twice; one shadow caster still lights
  every level, at the prior art of the sky easing and one-sun tests.
- **Sound**: bed volumes by `when` and Daylight; track choice by the 0.5
  threshold; a Soundscape with no `when` and no `night-music` mixes as today, at
  the prior art of the music tests in `tesserae-core`.
- **Declarations**: each validation rule refused with its field named, and a
  Declaration without the new fields loads as today, at the prior art of the
  contract validation tests.
- **Cloud layer**: the drift is the same for the same Time of day after a
  reopen, and the cloud field in metres is the same at another Voxel size.
- **Captures**, reviewed by the user before each look ticket lands: the baseline
  Eyes of the texture shimmer spec's ticket 01
  (`.scratch/2026-11-23-texture-shimmer/issues/01-baseline-captures.md`), the
  cave being its `cave` fixture, at noon, dusk, full-moon midnight and new-moon
  midnight, set with `TESSERAE_TIME_OF_DAY`, plus one Eye under the clouds and
  one above them.
- **Frame time**: GPU time from `scripts/profile-frames` on the `flight` fixture
  (M5, default render scale), interleaved against `main`, with the clock held at
  noon and at midnight. The sky's share (ticket 06) is at most 0.1 ms and the
  Cloud layer's (ticket 07) at most 0.2 ms; the whole, at most 0.3 ms, is
  checked in ticket 09.

## Out of Scope

- Weather: rain, storms, cloud cover that changes, cloud shadows, particles
  (step 24).
- Sleeping to skip the night; there are no beds.
- Pausing the World while the Menu is open.
- A darkness value gameplay reads, for spawning or crops: Mobs (step 25) defines
  it when it needs one.
- A clock Item, a command to set the time (step 29), and any WIT call for time.
- Seasons, latitude and a sun path that changes through a year.
- Fireflies and other night particles.
- Bevy's `Atmosphere`, auto-exposure, HDR and volumetric clouds: later Look-pack
  options
  ([ADR-0039](../../docs/adr/0039-the-sky-is-our-own-shader-not-bevys-atmosphere.md)).
- A separate dawn chorus or dusk period in the Soundscape.
- The other demo Games' skies: they declare no `sky` and keep noon.

## Further Notes

- The research behind this spec stays out of the history. Its findings that
  matter are stated here: other voxel engines also keep Sky light as exposure
  and scale it at draw time; a compressed night of 5 to 25% of noon is usual;
  Bevy's `Atmosphere` requires HDR and draws in physical units.
- DESIGN.md's LOD section described one camera per level, which ADR-0019
  retired; its camera and fog paragraphs were corrected with this spec. The
  dated "Verified constraints" section keeps its original wording.
- The day and night cycle finishes step 9's promise and is not a numbered step
  of its own; step 24 now depends on it.
