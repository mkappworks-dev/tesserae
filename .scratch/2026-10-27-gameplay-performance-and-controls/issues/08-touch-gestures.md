# 08: C4: Touch pointer ownership and actions

Status: ready-for-agent
Blocked by: 02, 06

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C4; one PR with ticket 09 (no overlay, nothing to try). Ticket 10
is validation only.

## What to build

Platforms: touch is delivered on Windows (WM_TOUCH/WM_POINTER) and Linux (X11
XI2, Wayland) only. winit 0.30 never emits `WindowEvent::Touch` on macOS, so
macOS is unclaimed and everything here is built and tested with injected
`TouchInput` messages. DESIGN.md's macOS-first order does not apply.

Give each native desktop touch id a role at start: Move, Look, Control (a
button) or Ui. The role never changes before end/cancel: a finger that slides
off a button still belongs to it, and one that began on UI never becomes
move/look when the UI closes. One contact per role; extra fingers in a full
role, and any beyond four, are ignored (count bound). Cancel every owned
contact, release its actions and clear removal progress on window focus loss,
a menu/catalogue/settings opening, and a switch to deliberate key, mouse or
pad use. A cancelled contact is not re-adopted; it must lift and touch again.

Touch is not a `Device` and has no binding set: its action wiring is fixed and
not remappable. It sets `Labels.device = Touch` for prompts only ("Tap Place",
no glyphs or key names). Deliberate use follows the pad's threshold.

Gameplay entry: the first deliberate touch on the world resolves
`Owner::Gameplay` with the cursor left unlocked and visible (a small change to
`input::resolve`). Mouse look stays gated on the lock. A deliberate cursor
lock switches back to mouse play. A touch never resumes play from `Modal`,
`Settings` or `Catalogue`.

Synthetic mouse: winit gives no flag, so suppress in our reader, never by
mutating `ButtonInput<MouseButton>`. Ignore mouse buttons and motion while any
touch is owned plus a grace period after the last contact ends (named
constant, 300 ms to start). A mouse event is real if it follows the quiet
period or arrives from a clearly different position. Include the final release.

Look: drag delta in logical px/s with its own sensitivity and invert-Y,
defaulting to the pad's values. Move: stick radius about 64 logical px and an
8 px dead zone, through the pad's throttle path (ADR-0008). No palm detection
of our own: only contacts beginning inside a control or zone count, there is
no tap-the-world action, and area/pressure are ignored.

Route through semantic actions and Server intents, never direct World access
or writes to raw mouse input.

## Acceptance criteria

- [x] Reordered move/look contacts preserve independent axes and identity.
- [x] Unknown/replaced ids cannot inherit removal progress.
- [ ] UI scroll/close/end/cancel cannot become a gameplay Edit.
- [x] Touch plus synthetic mouse yields at most one intended action,
      including the final release; later real mouse input works.
- [x] Focus loss, menu open and device switch cancel contacts with no Edit.
- [ ] Injected-`TouchInput` regressions pass at 60/144/1000 updates.
- [x] Touch play enters and leaves `Owner::Gameplay` without a locked cursor;
      mouse look stays gated on the lock.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices and the 2026-10-30 touch grilling are confirmed.
Local implementation is authorized; push, publication and merge are not.
Physical support is claimed only after ticket 10.

## Comments

### 2026-10-27: repository-based triage

Draft scope/acceptance require grilling; do not begin implementation.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above.

### World screens added by ticket 27 (2026-10-29)

Ticket 27 ships the World list, Create, Rename, Delete and Trust dialogs for
mouse and keyboard only, with Tab handling local to those screens. This
ticket's journeys must cover them; ticket 27 no longer claims that support.

### 2026-10-30: grilled; Agent Brief

Scope above is the confirmed result of the touch grilling. Implement with
ticket 09 in one PR. Earlier readiness statements are superseded.

### 2026-10-30: implementation notes (local, unpushed)

Implemented in `client/touch.rs` with injected-`TouchInput` regressions.
Deviations from the brief, for review:

- Touch play locks and hides the cursor on the first touch (as the pad does on
  South) instead of leaving it unlocked: six consumers gate on
  `CursorGrabMode::Locked`, so `input::resolve` needed no change.
- Mouse suppression is time-only (ticket 10 tunes `MOUSE_GRACE_S`); the
  "clearly different position" test is not implemented.
- `Labels.device` has no Touch state; only the cursor hint reads the touch
  frame. Contact count/role bounds and the 60/144/1000-update traces are not
  separately tested: contacts are event-driven, not per-update.

