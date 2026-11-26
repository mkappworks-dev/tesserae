# 03: Daylight cycle setting

Status: ready-for-agent
Blocked by: 01

Spec: [Day and night cycle](../spec.md).

## What to build

A Player turns the Daylight cycle on or off on New World and on Edit World,
beside Game mode.

- On by default; a World saved without the setting opens with it on.
- Off holds the Time of day at its current hour; on resumes from there.
- Edit World writes no World state: like Game mode, the toggle is a pending
  change the Server applies when the World next opens, and the screen says so.
- Create records in the World's sidecar whether its Game declares a `sky`;
  neither screen shows the toggle for a Game without one.
- The toggle lands in `mockup/` first and is approved before the Client
  implements it.

## Acceptance

- [ ] The mockup shows the toggle on both screens and the user approved it.
- [ ] A World created with the cycle off opens at the start hour and stays
      there.
- [ ] Turning it off in Edit World holds the hour from the next open; on
      resumes it; both survive a save and reopen.
- [ ] No toggle for a Game without `sky`.

## Comments

Built; tests written, not yet run (every test runs once at the end of the
spec), so the boxes stay unticked until then. The mockup waits on the user's
approval: view it in `mockup/app.js` (`cyclePicks`, used by `worldForm()` on New
World's World step, `reviewForm()` and `edit()`; `games.Terra.sky` marks the one
mock Game with a sky).

- Save: `Commit::daylight_cycle: bool` with
  `#[serde(default = "daylight_cycle_on")]`, so a commit without it runs.
  Always written. `Clock::cycle()` is what `drive` saves.
- Sidecar: `sky: bool` (written only when true) and `daylight_cycle: bool`
  (default on), mirrored from the commit at every open as `game_mode` is; a
  sidecar repaired from a commit takes `sky` from `time_of_day.is_some()`. The
  pending change is `ModeChanges::daylight_cycle: Option<bool>`, so the
  existing `mode_changes` key, `request_modes`, the unsaved-until-committed rule
  and the clear-on-commit all carry it unchanged.
- Server: `Persistence::prepare` picks the cycle from the pending change, else
  the commit, else the sidecar (a new World), else on, and calls
  `Clock::set_cycle` before `WorldClock` is made; `drive` saves
  `clock().cycle()`. No mutable `WorldClock` accessor was needed: nothing
  changes the cycle while the World runs.
- Client: `BiomePreview::sky` from the Game read (`mods::preview`), kept on
  `Form::sky` beside the Climate sizes; `Form::daylight_cycle`, `Act::Cycle`
  (New World) and `Act::EditCycle` with `Launcher::cycle` (Edit World),
  `edit_world::cycle(details)` for the value once a pending change applies.
  `cycle_picks` in `pages.rs` is an Off | On segmented control under Game mode;
  Left and Right step it. Review lists "Daylight cycle". New World with these
  settings copies it. `TESSERAE_LIST_SCRIPT` gains `cycle-off`.
- Wording: on, "The sun rises and sets, and the hours pass."; off, "The sky
  holds at its current hour."; a pending change, "Daylight cycle changes take
  effect when the World next opens." The discard prompt still says "name or
  mode"; left as is.
- Tests: `save/tests.rs`, `save/library.rs`, `tests/time_of_day.rs`,
  `server/persistence/tests.rs` (created off holds at the start hour; Edit
  World off then on, across saves and reopens) and `client/worlds/tests.rs`
  (Create records it, Left/Right, no toggle without a sky, Edit World's pending
  change and note).
