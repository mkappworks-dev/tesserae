# 01: Time of day on the Server

Status: ready-for-agent
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

- [ ] Clock tests: hours fixed to the sun under any day share, a night hour
      faster than a day hour at 0.7, a day wraps, Daylight's band, the Moon
      phase changes at noon only.
- [ ] A save and reopen keeps the hour; a World saved without it opens at the
      start hour; a Game with no `sky` is at noon.
- [ ] `TESSERAE_TIME_OF_DAY` holds the clock at the hour and night given.
- [ ] Each validation rule (Day length at least 60 s, start hour in [0, 24),
      day share in (0, 1)) is a load error naming the field; a Declaration
      without `sky` loads as today; only the Game may set `sky`.
