# 04: Sun and moon light

Status: ready-for-agent
Blocked by: 02

Spec: [Day and night cycle](../spec.md).
[ADR-0039](../../../docs/adr/0039-the-sky-is-our-own-shader-not-bevys-atmosphere.md).

## What to build

The light moves through the day and night on Terra, from the Sky state.

- `sky` gains the sun's tilt (toward the south, +Z), validated in [0, 60];
  Terra declares 25°.
- The Sky state, one pure function in `tesserae-core`: this ticket fills the
  sun and moon directions, the light, the ambient, the sky and fog colours and
  the Moon phase from the engine's default Day curve.
- The Server's sky message gains a flag for a Cave Biome's air; the Client
  keeps the eased pair in its own resource and writes the Sky state's colours
  to `FogSettings` and the clear colour. Cave air is never tinted.
- One shadowed `DirectionalLight`: the sun while it is up, the moon while it is
  down, its strength times the horizon fade whatever the curve says, so zero
  where it swaps. Moonlight scales with the Moon phase.
- The sky's sun disc reads the sun's direction from the Sky state, not from the
  light, and is not drawn below the horizon.
- The ambient dome dims at night; a face that sees no sky keeps the noon floor
  from its own uniform, so a sealed cave is the same at every hour. Block light
  is unchanged.
- Underwater fog and the clear colour behind it dim and tint with the ambient;
  an emitting fluid keeps its own.
- `DistanceFog` gains its glow toward the light, from the light's colour.

## Acceptance

- [x] Sky state tests: sunrise and sunset at 06:00 and 18:00, the noon sun the
      tilt toward the south, zero light on both sides of the swap whatever the
      curve, full moon brighter than new, cave air untinted, no `sky` is
      today's noon.
- [x] One shadow caster still lights every level (the one-sun test holds).
- [ ] Captures at the baseline Eyes at noon, dusk, full-moon and new-moon
      midnight, approved by the user; the `cave` fixture's captures at noon and
      midnight match; the map at midnight is the map at noon.

## Comments

Built; tests written, not yet run (every test runs once at the end of the
spec), and no captures taken, so the boxes stay unticked until then.

- Contract: `sky-params.tilt-deg` (required, last), data-only `tilt-deg`,
  `SkyParams::tilt_deg`, error field `sky.tilt_deg`; Terra declares 25.
  `sun_elevation_deg(hour, tilt_deg)` now takes the tilt, and `Clock` and
  `ClientTimeOfDay` Daylight use it.
- Sky state, in `tesserae_core::sky` (`sky/state.rs`):
  `sky_state(&SkyInputs) -> SkyState`, which is
  `sky_state_on(&DEFAULT_DAY_CURVE, ..)`. Ticket 05 picks the Game's curve
  there; `sky_state_on` also lets a test pass any curve.
  - `SkyInputs { hour, moon_phase, sky: Option<&SkyParams>, air: Air }`, with
    `Air { sky, fog, underground }`: the eased pair in sRGB, and `underground`
    1 for a Cave Biome's air (a share, not a flag, so the Client eases it with
    the colours and the tint fades as the air turns). Ticket 07 adds the cloud
    cover and the time since day 0; Weather adds its inputs here too.
  - `SkyState { toward_sun, toward_moon, pole, light_source: LightSource
    (Sun | Moon), toward_light, light: Light, ambient: Light, sky, fog, stars,
    star_turn_rad, moon_phase, moon_fullness, daylight }`. East is +X, south
    +Z; the sun's path is `[cos a, sin a cos t, sin a sin t]` with `a` the
    angle since 06:00 and `t` the tilt, the moon opposite, both turning about
    `pole = [0, -sin t, cos t]`. Ticket 06 draws the moon at `toward_moon` with
    `moon_phase`/`moon_fullness`, and the stars by `stars`, rotated
    `star_turn_rad` about `pole`. Ticket 07 adds cloud cover and drift.
  - `Light { colour (linear), strength (share of noon) }`, `Light::NOON`,
    `Light::shade(srgb)`. `Tint { colour (linear), amount }`, `Tint::NONE`,
    `Tint::apply(srgb)`: a lerp in linear light; amount 0 returns the input
    to the bit. Keys ease tints premultiplied, so a key with no tint lends
    the ease no colour.
  - `DayKey { hour, sun, moon (at full moon), ambient, sky_tint, fog_tint,
    stars }`, `day_curve_at(keys, hour)`: linear, wrapping at midnight.
    `DEFAULT_DAY_CURVE`: night at 05:00 and 19:00 (moon 0.15, ambient 0.25
    blue, sky and fog tinted 0.92 toward navy, stars 1), twilight at 06:00 and
    18:00 (warm sun, ambient 0.55, fog tint 0.55 orange, sky tint 0.4, stars
    0.2), today's noon at 08:00 and 16:00. A first guess for captures; Terra's
    own curve is ticket 05's.
  - `CAVE_FLOOR` (0.0642, was the shader's `UNLIT_AMBIENT`) moved to core, for
    ticket 05's validation; the default curve's test keeps every key above it.
  - The light's fade is 0 at the horizon and 1 at 10° (not Daylight, which is
    0.5 at the horizon); the moon's strength is the curve's times
    `moon_fullness`, `(1 + cos(2π phase / 8)) / 2`. No `sky` is today's fixed
    light (toward `(1, 2.2, 0.8)`), noon colours, the pair untouched.
- Server: `SkyFogTarget::cave`, true for a Cave Biome's air; a change of it
  alone is resent.
- Client (`client/sky.rs`): `EasedAir(Air)` is the eased pair (session state,
  starting at the haze), never tinted; `SkyNow(SkyState)` is the state each
  frame. `present` (after the clock, before `fog::follow_settings`) writes
  `FogSettings.sky/color`, the new `FogSettings.glow` (the light's colour,
  `None` without a sky), `GlobalAmbientLight` (`SKY_AMBIENT` × strength, its
  colour), and the one `DirectionalLight` (`SUN_LUX` × strength, colour,
  transform toward the light rolled about the pole so the shadow map never
  spins), each only when it changed.
- `DistanceFog.directional_light_color` is the glow colour with alpha 0.08
  (`GLOW_SHARE`); Bevy already scales it by the light's lux, so it fades at
  night. `sky.wesl` adds the same glow to its horizon (`fog_glow`, Bevy's
  `apply_fog` formula), so the reach's edge stays hidden toward a low sun.
- Sky shader: no new uniform; `SkyMaterial.toward_sun` now comes from
  `SkyNow`, not the light. The disc is cut at the horizon per pixel; its glow
  fades with the sun's height (gone 6° below), so no line is drawn across the
  sky seen from above the reach.
- Cave floor: no per-frame material uniform, since changing the chunk
  materials every frame would re-prepare them and re-specialise every Chunk.
  Shader defs `CHUNK_CAVE_FLOOR` and `CHUNK_NOON_AMBIENT` (bits of
  `CAVE_FLOOR` and `SKY_AMBIENT`) give the noon floor; `cave_floor()` divides
  it by Bevy's own `lights.ambient_color` per channel, which Bevy multiplies
  back in, so a sealed face gets the noon floor at every hour, untinted.
  Block light is unchanged.
- Underwater: `FluidSurface::emits` (the fluid's emission above 0). A fluid
  that does not emit has its hue `SkyNow`'s ambient `shade`d before it
  becomes the fog and the clear colour; lava keeps its own. A change of the
  ambient repaints while under.
- Decided: `ClientTimeOfDay::daylight` keeps its `expect(dead_code)` (ticket
  08's sound reads it, or `SkyNow`'s `daylight`); `sky` and `moon_phase` lost
  theirs. Entities and dropped items in a cave dim at night (they have no
  Sky light, so no floor); the hand and icons are unlit. The map is
  unchanged: it draws captured colours.
- Tests: `crates/tesserae-core/tests/sky_state.rs`, tilt cases in
  `tests/time_of_day.rs` and `tesserae-mods` `declare.rs`; Client follow
  tests in `client/sky.rs` (`day_and_night`: dusk, tinted once, cave at noon
  and midnight, the one light is the moon at midnight, no sky is noon),
  `client/underwater.rs` (night water, lava), `render/sky.rs` (the sun from
  the state while the light is the moon), `render/fog.rs` (glow), and
  `server/sky.rs` (the cave flag).

The tests pass in the full `scripts/cargo-ci` run of 2026-11-26.
