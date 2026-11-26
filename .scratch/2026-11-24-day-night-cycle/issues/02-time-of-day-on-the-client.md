# 02: Time of day on the Client and in the Menu

Status: ready-for-agent
Blocked by: 01

Spec: [Day and night cycle](../spec.md).

## What to build

The Client knows the hour and shows it.

- The Server sends the Time of day and its rate when the World opens, when the
  rate changes, when the time is set and every 5 s. The Client advances it
  locally, eases a correction over about 2 s and snaps one of more than a
  quarter day. The Client never sets it.
- The Menu's "You are here" adds "Day N, HH:MM", the first day being Day 1. The
  line lands in `mockup/` first and is approved before the Client shows it.

## Acceptance

- [ ] Server tests: the message on open, on a rate change and every 5 s.
- [ ] Client tests: a small correction eases, a large one snaps, the clock runs
      between messages at the sent rate.
- [ ] The mockup line approved by the user; the Menu shows it on Terra.

## Comments

Built; tests written, not yet run (every test runs once at the end of the
spec), so the boxes stay unticked until then. The mockup line waits on the
user's approval: view it in `mockup/app.js` (`pause()`, the in-game Menu) and
`mockup/style.css` (`.world-status .world-time`).

- Messages, in `crate::content`: `WorldSky(Option<SkyParams>)`, the Game's
  whole `sky` record, sent once on the first publish; and
  `TimeOfDaySync { time: TimeOfDay, steps_per_s: f32 }`. Both come from
  `server::time_of_day::publish`, which reads `WorldClock`, so the sky the
  Client reads is the record the Server's clock runs on. Tickets 04, 05 and 07
  extend `SkyParams` and it travels as is.
- Resend rule: on the first tick, when the rate changes, every 5 s, and when
  the time is more than 1 s of the sent rate (half a step while held) from
  where the last message put it. That last one is "when the time is set": no
  hook is needed when step 29 adds a command, and a simulation that falls
  behind is corrected early too. Seconds are virtual time, as the
  simulation's steps are.
- Client: `crate::client::time_of_day::ClientTimeOfDay`, a session-state
  resource: `sky()`, `steps()` (fractional), `time()`, `day()` (from 0),
  `hour()` (fractional between steps), `daylight()`, `moon_phase()`,
  `label()`. A Game with no `sky` reads noon, day 0, phase 0. `sky`,
  `daylight` and `moon_phase` carry `expect(dead_code)` outside tests until
  ticket 04 reads them; remove it then. The easing is `Follow`, Bevy-free:
  `advance(dt)` and `correct(time, rate, snap_beyond)`; a correction lands
  linearly in 2 s; one more than a quarter day away, or the first, snaps.
- Menu: "Day N, HH:MM" sits between the Biome and the Game line; only that
  text updates while the Menu is open. A Game with no `sky` shows no line,
  since it has no day to tell (the spec left this open).
