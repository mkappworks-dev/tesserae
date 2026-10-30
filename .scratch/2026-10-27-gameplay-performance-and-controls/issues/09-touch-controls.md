# 09: C4: Touch overlay and preferences

Status: ready-for-agent
Blocked by: 08, 04

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C4; one PR with ticket 08. If it grows too big, split the
settings UI out, never 08 from 09.

## What to build

Intentional touch reveals the optional overlay; deliberate key, mouse or pad
use hides it, so mouse play keeps no permanent overlay. Fixed explicit
controls only; no pinch, double-tap or long-press-on-world gestures.

- Left zone: virtual stick (move). Right zone: look.
- Buttons: Place, Remove (hold), Use, Sample, Ascend, Descend, Sprint toggle,
  slots, Shape/Turn/Flip, empty hand, view/mode and catalogue, as those
  capabilities exist. Undo/Redo only where history is available.
- A Menu button in a top corner, outside the thumb zones, opens the
  pause/settings menu: the touch route to catalogue, settings and the World
  list. Every screen has a close button (there is no Escape). Help and
  diagnostics stay keyboard-only.

Remove is hold-to-remove through the existing `Remove` action, progress and
`tap_remove`/`repeat_held` preferences unchanged. Lifting or sliding off
cancels. A button's active area is its drawn size. Place and Remove are
separate buttons; Place is tap or hold under the same repeat rule.

Catalogue and menus without hover or drag: tap selects an entry and shows
what hover shows, a separate "Assign to slot" control picks the slot by tap.
Lists scroll by one-finger drag with no inertia; a drag past 6 logical px
cancels the tap, so scrolling never selects or assigns. Mouse hover and
drag-assign are unchanged. The same applies to the World list, Create, Rename,
Delete and Trust screens. Text entry uses the physical keyboard; no on-screen
keyboard.

Layout: logical px times `UiScale`. Targets at least 44 px with at least 8 px
gaps. Default is stick bottom-left, look right, Place/Remove and primary
buttons bottom-right in the lower third of each side. The centre stays clear
for the crosshair and Server target/progress cues. No controls in the top 15%
except Menu. Presets: normal and mirrored (mirroring swaps Place/Remove). Size:
small, normal, enlarged. Opacity 30–100%. Touch look sensitivity and invert-Y
sit in the same settings section. All respect Apply/Cancel/reset. No freeform
HUD editor.

## Acceptance criteria

- [ ] Play and catalogue operations require no hover or keyboard numbers.
- [ ] Catalogue assignment works by tap alone; scrolling never selects.
- [ ] A touch-only player can open the menu, change settings and leave play.
- [ ] Gestures never leak through overlay/UI transitions.
- [x] Preset/size/opacity/look settings respect Apply/Cancel and reset
      (they reuse the controller page's draft and reset path; no new test).
- [x] Controls are at least 44 px with 8 px gaps at every size and scale.
- [ ] Retain normal/small/enlarged native overlay captures for review.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review,
uncommitted. All Q1–Q49 product choices and the 2026-10-30 touch grilling are
confirmed. Local implementation is authorized; push, publication and merge are
not. Physical support is claimed only after ticket 10.

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

### 2026-10-30: grilled; Agent Brief

Scope above is the confirmed result of the touch grilling. Implement with
ticket 08 in one PR. Earlier readiness statements are superseded.

### 2026-10-30: implementation notes (local, unpushed)

Overlay and settings are in `client/touch.rs` and the controller settings
page. Open items:

- The Menu button is `Action::Help` (it opens settings); Items is
  `Action::Catalogue`. Every window already has its own close button.
- Catalogue tap-select and Assign to slot already run through `Activate`,
  which picking raises for touch; scroll-drag cancel (6 px) is not added.
  Both need ticket 10 to confirm on hardware.
- Normal/small/enlarged captures are not retained yet.

