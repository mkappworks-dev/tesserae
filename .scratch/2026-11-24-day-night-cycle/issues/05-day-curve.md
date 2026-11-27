# 05: Day curve

Status: done
Blocked by: 04

Spec: [Day and night cycle](../spec.md).

## What to build

A Game declares its Day curve and Terra declares its own, so dawn, noon, dusk
and night look the way the Game wants.

- `sky` gains the Day curve: keyframes at hours, each with the sun's and moon's
  colour and strength, the ambient's colour and strength, a sky tint and a fog
  tint (a colour and an amount) and star brightness. Eased linearly, wrapping
  at midnight. The data-only TOML, the starter's contract and the
  compatibility doc change alongside.
- A tint moves the Biome colour toward the tint colour by the amount, in linear
  light.
- Terra declares a first curve: warm at dawn and dusk, blue at night, about 15
  to 20% of noon at full moon on open ground and about 8% at new moon.

## Acceptance

- [x] Each rule (1 to 16 keys, hours ascending in [0, 24), strengths, amounts
      and colours in [0, 1], ambient at every key at least the cave floor) is a
      load error naming the field.
- [x] A key with amount 0 returns a Biome pair unchanged; a `sky` without a
      curve uses the engine's default.
- [x] Terra's curve approved by the user on captures at noon, dusk, full-moon
      and new-moon midnight.

## Comments

Built; tests written, not yet run (every test runs once at the end of the
spec), and no captures taken, so the boxes stay unticked until then.

- Contract: `sky-params.day-curve: option<list<day-key>>` (last);
  `day-key { hour, sun, moon, ambient: curve-light, sky-tint, fog-tint:
  curve-tint, stars }`, `curve-light { colour: tuple<f32, f32, f32>, strength }`,
  `curve-tint { colour, amount }`. Data-only: `[[world.sky.day-curve]]` rows,
  `sun = { colour = [r, g, b], strength }`, `sky-tint = { colour, amount }`,
  every field required. The v0.2 adapter needs nothing: it sets `sky: none`.
- Core: `SkyParams::day_curve: Option<Vec<DayKey>>` and
  `SkyParams::day_curve()` (the declared keys or `DEFAULT_DAY_CURVE`);
  `sky_state` reads it, so the Client needed no change. `SkyParams` and `Clock`
  are no longer `Copy`; call sites clone (`WorldClock::new`,
  `Persistence::prepare` and `install`, tests).
- Validation: `SkyParams::validate` returns `SkyError { key, source }`, whose
  `field()` is `sky.tilt_deg` or `sky.day_curve[2].hour`; a new
  `ModError::Sky` prints it as the other `world` errors are. Rules: 1 to 16
  keys (`sky.day_curve`); each hour in [0, 24) and after the key before it;
  every colour channel, strength, amount and `stars` in [0, 1]; the ambient
  strength at least `CAVE_FLOOR`.
- Decided: the cave floor rule checks the ambient strength, as the spec says,
  not each channel of colour times strength (the default curve's test does
  that); a deep blue ambient may so dip under the floor in red. Terra's keys
  clear it per channel too.
- Terra (`games/terra/src/sky.rs`), ten keys: night at 04:30 and 19:30
  (ambient 0.38 `(0.55, 0.66, 1.0)`, moon 0.19 `(0.62, 0.74, 1.0)`, sky and
  fog tinted 0.92 and 0.9 toward navy, stars 1); blue hour at 05:30 and 18:30
  (ambient 0.5, sky 0.7 toward deep blue, fog 0.65 toward violet, stars 0.5);
  sunrise and sunset at 06:00 and 18:00 (sun `(1.0, 0.5, 0.24)`, ambient 0.6
  warm, fog 0.6 toward orange, sky 0.4 toward dusky violet, stars 0.1); golden
  hour at 07:00 and 17:00 (sun `(1.0, 0.8, 0.58)`, ambient 0.85, fog 0.25
  toward amber); noon from 08:30 to 15:30. By the engine's noon (3600 lux sun,
  1500 lux sky, a top face under a 25° tilt) and each colour's luminance,
  midnight is about 8% of noon at new moon and 17% at full. A first pass for
  the captures.
- Tests: `tesserae-core/tests/sky_state.rs` (no curve reads the default, a
  declared curve is used, a declared key with amount 0 leaves the pair to the
  bit, the default curve passes validation, each rule names its field and the
  edges pass); `time_of_day.rs` reads `field()`; `tesserae-mods`
  `tests/declare.rs` (a curve is kept, no curve is the default, bad keys name
  the key and field through `ModError::Sky`) and `tests/data.rs` (`full.toml`
  carries two keys; a bad `stars`, a descending hour and an unknown field).

The tests pass in the full `scripts/cargo-ci` run of 2026-11-26.

2026-11-26: Terra's night is darker at the user's request: moonlight 0.09
(was 0.19) and night ambient 0.18 (was 0.38). With the night ambient now
following the Moon, full-moon midnight is about 12% of noon on the ground
(`near-full-midnight-v4`) and new-moon midnight sits on the cave floor
(`near-new-midnight-v4`).

The user approved the look on the captures on 2026-11-26.
