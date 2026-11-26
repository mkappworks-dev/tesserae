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
