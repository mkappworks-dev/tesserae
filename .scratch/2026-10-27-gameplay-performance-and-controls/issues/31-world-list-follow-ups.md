# 31: W0.1: World list window and keyboard follow-ups

Status: needs-triage
Blocked by: 27

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: W0.1; keep this separate from other PR scopes.

## What to build

Split from ticket 27's acceptance criteria on 2026-10-29.

- Carry the window position across a restart even when the window never
  moved. winit sends no position until the first move, so read it another
  way or accept centring.
- Carry macOS native fullscreen (the green button), which Bevy's
  `WindowMode` does not reflect.
- Check by hand that Tab, Shift-Tab and Enter work on every World list
  screen. No input automation was available when ticket 27 shipped.

## Acceptance criteria

- [ ] A restart keeps an unmoved window's position, or the ticket records
      why centring is accepted.
- [ ] A restart from native fullscreen stays fullscreen.
- [ ] Tab, Shift-Tab and Enter are verified natively on the list, Create,
      Rename, Delete and Trust screens.
