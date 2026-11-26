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
