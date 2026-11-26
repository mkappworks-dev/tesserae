# 10: Creative Time of day

Status: done
Blocked by: 02, 03

Spec: [Day and night cycle](../spec.md).

## What to build

A Player in Creative sets the Time of day from the Menu.

- A "Time of day" item in the Menu, after Map, only in Creative and only for a
  Game with a `sky`. It opens a dialog as `mockup/app.js` (`timeDialog`,
  `timeThumb`, `customRow`) and `mockup/style.css` (`.time-pick`,
  `.time-custom`) show it: four tiles with a scene of each sky and where each
  lands ("Day 4, 06:00"), a Custom row stepping a quarter hour and taking a
  typed `HH:MM`, Cancel and Set time.
- The Client sends a request; the Server checks Creative and a `sky`, moves the
  clock forward to the next time it reads that hour, saves it with the World
  and sends the Time of day at once.
- With the Daylight cycle off the clock holds at the new hour.

## Acceptance

- [x] Server: Creative moves the clock forward to the hour, today if it is still
      ahead, else tomorrow; the Moon phase follows the day.
- [x] Server: a Survival Player, or a World with no `sky`, is refused and the
      clock does not change.
- [x] Server: the new time survives a save and reopen; with the cycle off it
      holds there.
- [x] Client: the Menu item shows only in Creative with a `sky`; tiles,
      Custom stepping and typing, and Set time send the request.
- [x] The Menu's "Day N, HH:MM" line shows the new time after Set.

## Comments

The user approved the mockup on 2026-11-26, after asking for the custom field
and for a scene in each thumbnail.

Built, with tests for the first four boxes; the last waits on seeing it in the
running game (`a_tile_and_set_time_ask_for_its_hour_and_the_menu_shows_the_time_sent_back`
covers the line with the Server's answer stood in).

- Core (`tesserae-core/src/sky.rs`): `TimeOfDay::next_at(minute, sky)` lands
  today while the minute is ahead, else tomorrow, through `on_day`, stepping on
  at most a few steps so it reads back as that minute (the Menu floors);
  `TimeOfDay::minute`; `Clock::set_forward(minute)`, `false` with no sky or a
  minute past 23:59; `MINUTES_PER_DAY`. Tests in `tests/time_of_day.rs`.
- Server (`server/time_of_day.rs`): `SetTimeOfDay { minute }`, a Player intent;
  `set` in `FixedPreUpdate` before `advance` grants it only to a Player whose
  `PlayerMode` is not Survival, and `Clock::set_forward` refuses a World with no
  sky. Refusals are silent: the Client offers the item only where it is
  granted. `publish` already sends a time that moved at once.
  `WorldClock::revision` counts the sets; `Persistence::saved_clock` makes a set
  clock dirty until a commit holds it (else a held clock set in an otherwise
  clean World would wait for Save & Quit). Tests beside the existing ones in
  `time_of_day.rs` and `persistence/tests.rs`
  (`a_time_set_in_creative_is_unsaved_until_a_save_holds_it_and_survives_a_reopen`).
- Client (`client/time_dialog.rs`): `Choice::TimeOfDay` after Map in
  `menu::list`, only in Creative with a sky; `Menu::time` opens the dialog,
  built once and kept in step by `refresh` (landings follow
  `ClientTimeOfDay`, the note follows `ClientTimeOfDay::runs`). Tiles are drawn
  from `theme::TIME_SCENES` with a `LinearGradient` sky and node rectangles for
  the stepped hills, sun, glow and stars; the Custom row's field commits on
  Enter or losing focus, reverting with "Type a time from 00:00 to 23:59.".
  Escape or the pad's back closes the dialog only and focus returns to the
  item. Set time toasts "Time set to sunset, Day 4, 18:00.". Tests in
  `menu.rs` (`tests::time_of_day`) and `time_dialog.rs` (`parse_hhmm`).
- Not this ticket: `a_reopened_world_keeps_the_hour_it_was_saved_at` fails on
  `cba14878` without this change too: the commit it reads holds the start hour,
  likely the open-time save, which its settled save waits out without saving.

The user saw it in the running game on 2026-11-26.
