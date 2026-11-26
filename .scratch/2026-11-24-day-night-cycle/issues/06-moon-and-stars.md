# 06: Moon and stars in the sky

Status: ready-for-agent
Blocked by: 04

Spec: [Day and night cycle](../spec.md).
[ADR-0039](../../../docs/adr/0039-the-sky-is-our-own-shader-not-bevys-atmosphere.md).

## What to build

The night sky.

- A moon disc at the Sky state's moon direction, showing its Moon phase, not
  drawn below the horizon.
- Procedural stars from the view direction, rotating with the Time of day,
  fading in with the star brightness.

## Acceptance

- [x] Captures at dusk and at full-moon and new-moon midnight, approved by the
      user; the moon's phase reads at a glance.
- [ ] The reach's edge stays hidden at every hour.
- [x] GPU frame time of the sky on `flight` at noon and midnight, interleaved
      against `main`, at most 0.1 ms over it, recorded in Comments.

## Comments

Built; tests written, not yet run (every test runs once at the end of the
spec), and no captures or frame times taken, so the boxes stay unticked.

- Uniforms: `SkyMaterial` gains `toward_moon`, `moon_light`,
  `stars_from_world` (a `Mat4`) and `stars`, all from `SkyNow` through
  `SkyMaterial::new(sky, &state)`. `MOON_PHASES` in `tesserae_core::sky` is
  now public.
- Moon: the moon stands opposite the sun, so a real phase angle would always
  be full. `moon_light(state)` is toward the sun at full and away at new, and
  between them swings along the moon's path (`pole × toward_moon`): ahead
  while waxing (5 to 7), as the set sun is, behind while waning (1 to 3). The
  shader lights the disc as a sphere from it, with a narrow terminator and a
  little limb darkening; the dark part is the sky without stars, so a new moon
  hides stars but shows nothing. A faint glow scales with how full it is.
  Radius 0.014 rad, about three times the real moon. No maria.
- Stars: the view direction turned into the stars' frame (about `pole` by
  `-star_turn_rad`), then one cell of an equi-angular cube map, 90 cells a
  face (about 1°); a PCG hash of the cell gives 16% of cells a star, its place
  (kept 15% of a cell from the edges, so a pixel reads only its own cell),
  brightness (mostly faint) and a slight warm or blue tint. Three hashes, two
  `atan` and two `tan` per pixel at most.
- No shimmer: a star's light is a smooth bump reaching 2 pixels, sized from
  the projection and viewport height, so whatever the resolution it covers
  a few pixels and keeps its brightness as it turns across the pixel grid.
- Fog and the reach: stars are scaled by the horizon band's own `up`, so they
  are gone where the sky is the fog colour; the moon and its glow fade by a
  steeper haze (half gone 1° up) and nothing of either is drawn at or below
  the horizon. The shader returns before any of it below the horizon, and
  skips the stars when `stars` is under 0.004 and the moon outside its disc
  and glow.
- Clouds (ticket 07) need no hook: they draw over the sky in the blended
  pass, so they hide the stars and the moon.
- Cave floor fix: validation now checks each channel of the ambient colour
  times its strength against `CAVE_FLOOR`, error field
  `sky.day_curve[i].ambient`; the test in `sky_state.rs` has a black channel
  case and its edge key is white at the floor, and `tesserae-mods`'
  `declare.rs` names the new field. The WIT doc (both copies), `data-mods.md`
  and `contract-compatibility.md` state the rule per channel. The shader's
  division in `cave_floor()` already had `max(…, MIN_AMBIENT)` (1e-3). The
  default curve's lowest channel is 0.6 × 0.25 = 0.15 and Terra's
  0.55 × 0.38 = 0.21, both above 0.0642; `full.toml`'s 0.65 × 0.55 too.
- Tests (`render/sky.rs`): full moon lit from the sun and new from behind;
  waxing ahead and waning behind along the path, a crescent mostly behind the
  face; the sun keeps its place among the stars all day; the material follows
  `SkyNow`'s moon and stars. No shader validation test or naga tool exists in
  the repo, so `sky.wesl` is read, not compiled, until the first run.

The tests pass in the full `scripts/cargo-ci` run of 2026-11-26.

2026-11-26: the night's ambient follows the Moon. Its night share falls with
the phase to `STARLIGHT_SHARE` (0.35) at new moon, never under the cave floor
(`night_ambient` in `sky/state.rs`). A full moon high up takes `MOON_WASH`
(0.15) off every star in `sky.wesl`, so the faint two thirds vanish and the
bright ones dim. Captures: `far-full-midnight-v2`, `far-new-midnight-v2`,
`sky-full-midnight`, `sky-new-midnight`.

The user approved the look on the captures on 2026-11-26.

2026-11-26, frame time. The first A/B put the sky without clouds 0.51 ms over
no sky on `flight` at noon: bevy_pbr's fog samples the shadow map again for its
sun glow, a second full shadow lookup on every terrain pixel. The chunk shader
now fogs the terrain itself with the glow unshadowed, as the sky and the clouds
do (45c1e246). Measured 2026-11-26 at 1920×1080, release with `profiling`, Terra, rounds
alternated with `main` (b6817a1b) under a load of 6 to 7; median of the
per-run p50 GPU frame (`frame_start` to `frame_end`), 6 runs a cell.

| `flight`, GPU frame        | Round 1 | Round 2 |
| -------------------------- | ------- | ------- |
| Terra without its sky      | 4.545   | 4.494   |
| The sky, clouds off        | 4.466   | 4.517   |

The sky's share is level with none, within the 0.1 ms. The reach's edge is
checked in ticket 09's captures.
