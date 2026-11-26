# 01: Time of day on the Server

Status: done
Blocked by: None (can start immediately)

Spec: [Day and night cycle](../spec.md).

## What to build

A World has a Time of day the Server owns, advances and saves, which a capture
can pin, before anything in the sky moves.

- `world-params` gains `sky`, which only the Game may set, with the Day length,
  the start hour and the day share, and their validation. The data-only TOML,
  the Rust starter's copy of the contract and the compatibility doc change
  alongside. The other `sky` fields arrive with tickets 04, 05 and 07.
- In `tesserae-core`, with no Bevy: the clock as a count of logical steps since
  day 0, advanced one per logical step while the Daylight cycle is on; the hour
  fixed to the sun through the day share (sunrise 06:00, sunset 18:00); the
  sun's elevation and Daylight (0 below -10°, 1 above +10°, smoothstep
  between); the Moon phase, which changes at noon, night 0 full.
- Saved with the World; a World saved without it opens at the Game's start
  hour. A Game with no `sky` is held at noon.
- Under `dev` and `profiling` the Server reads `TESSERAE_TIME_OF_DAY`
  (`"19:30"`, `"4 00:00"`) and holds the clock there.
- Terra declares a 20-minute day, a 0.7 day share and a 07:00 start.

## Acceptance

- [x] Clock tests: hours fixed to the sun under any day share, a night hour
      faster than a day hour at 0.7, a day wraps, Daylight's band, the Moon
      phase changes at noon only.
- [x] A save and reopen keeps the hour; a World saved without it opens at the
      start hour; a Game with no `sky` is at noon.
- [x] `TESSERAE_TIME_OF_DAY` holds the clock at the hour and night given.
- [x] Each validation rule (Day length at least 60 s, start hour in [0, 24),
      day share in (0, 1)) is a load error naming the field; a Declaration
      without `sky` loads as today; only the Game may set `sky`.

## Comments

Built; tests written, not yet run (every test runs once at the end of the
spec), so the boxes stay unticked until then.

- Core: `tesserae_core::sky` holds `SkyParams` (`day_length_s`, `start_hour`,
  `day_share`, `validate`, `day_steps`), `TimeOfDay` (steps since day 0's
  midnight: `on_day`, `day`, `hour`, `moon_phase`), `sun_elevation_deg(hour)`
  (no tilt yet: ticket 04 adds it), `daylight(elevation_deg)`, `Pin` (parses
  `TESSERAE_TIME_OF_DAY`) and `Clock` (`open`, `step`, `set_cycle`, `hold`,
  `runs`, `saved`, `hour`, `moon_phase`, `daylight`, `steps_per_s`). Tests in
  `crates/tesserae-core/tests/time_of_day.rs`.
- Night `n` starts on day `n`'s evening, so a World's first night is full and
  `"4 00:00"` is day 5's midnight, the new moon. Noon is always step
  `day_steps / 2`.
- Server: `crate::server::time_of_day::WorldClock` (a session resource, read
  with `clock()`) catches up with `FluidSim::now()` in `FixedPreUpdate`, after
  `simulate` and before `persistence::drive`. `Persistence::prepare` opens the
  `Clock` from `LoadedMods::sky` and the commit; `persistence::install` inserts
  `WorldClock::opened`, which applies the pin under `dev` and `profiling`.
- Ticket 03: the Daylight cycle is `Clock::set_cycle`; nothing saves it yet and
  `WorldClock` has no mutable accessor yet. Ticket 02: `Clock::steps_per_s` is
  the rate (64, or 0 while held or off).
- Save: `Commit::time_of_day: Option<u64>`, written only with a sky. It never
  marks the World unsaved (it changes every step), so the hour saved is the
  last commit's; Save and Quit always commits.
- Contract: `world-params.sky: option<sky-params>`, data-only `[world.sky]`,
  Game-only (`world.sky` in `beyond_character`); errors are
  `what: "world"`, fields `sky.day_length_s`, `sky.start_hour`, `sky.day_share`.

The tests pass in the full `scripts/cargo-ci` run of 2026-11-26.
