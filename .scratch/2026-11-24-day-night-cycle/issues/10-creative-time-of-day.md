# 10: Creative Time of day

Status: ready-for-agent
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

- [ ] Server: Creative moves the clock forward to the hour, today if it is still
      ahead, else tomorrow; the Moon phase follows the day.
- [ ] Server: a Survival Player, or a World with no `sky`, is refused and the
      clock does not change.
- [ ] Server: the new time survives a save and reopen; with the cycle off it
      holds there.
- [ ] Client: the Menu item shows only in Creative with a `sky`; tiles,
      Custom stepping and typing, and Set time send the request.
- [ ] The Menu's "Day N, HH:MM" line shows the new time after Set.

## Comments

The user approved the mockup on 2026-11-26, after asking for the custom field
and for a scene in each thumbnail.
