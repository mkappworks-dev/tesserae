# 10: C4: Real desktop touch acceptance

Status: done
Blocked by: 09

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: validation record only; no product code.

## What to build

Run two-finger move/look, controls, holds, catalogue browse/assignment,
interruptions, focus loss and mouse return on a Windows or Linux machine with
a real touchscreen, supplied by the user. macOS cannot produce touch events
(winit 0.30), and mobile emulation or this Mac is no substitute.

Record per run: OS and version, device, display scaling, X11 or Wayland,
whether touch arrives as `TouchInput`, whether the OS also emits synthetic
mouse events and when, and whether the grace constant from ticket 08 needed to
change (adjust it only from this evidence). Record visual approval
independently from physical validation.

Journeys: move and look together; Place, Remove hold and repeat-while-held;
Use and Sample; slots; tap-to-select and Assign to slot in the catalogue;
World list screens; Menu button and close; palm or resting-hand accidents;
focus loss mid-hold; menu open mid-gesture; return to mouse and keyboard.

## Acceptance criteria

- [ ] Actual contacts and synthetic mouse behavior match injected regressions.
- [ ] No accidental/duplicate Edit across focus or gesture interruption.
- [ ] Controls remain reachable/readable at the specified logical sizes.
- [x] Untested platforms/hardware remain explicitly unclaimed.
- [ ] Until this passes, README, spec and tickets say touch is "implemented,
      not validated".

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Native captures/measurements stay in one unique /tmp folder until review.
Needs a human with the hardware; an agent cannot complete it. Publication and
merge are not authorized from this status.

## Comments

### 2026-10-27: repository-based triage

Completion requires real desktop touch hardware and physical input evidence.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.

### 2026-10-30: grilled

Scope above is the confirmed result of the touch grilling. Status is
ready-for-human: it waits on ticket 09 and a user-supplied touchscreen.

### 2026-10-30: closed without hardware

Closed as done at the user's direction. No Windows or Linux touchscreen run
was made, so the other acceptance criteria are not met and stay unticked.
Touch is still "implemented, not validated": no physical contacts, emulated
mouse behaviour or `MOUSE_GRACE_S` tuning has been recorded, and no platform
is claimed as supported. Reopen this ticket when a touchscreen is available.

