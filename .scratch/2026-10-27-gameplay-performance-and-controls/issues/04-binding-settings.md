# 04: C1: Controls UI and configurable gestures

Status: done
Blocked by: 03

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: combined PR 2 (tickets 01–04 and 28).

## What to build

Expose Controls from catalogue/help. Capture begins after the opening input
is released; Escape remains a cancellation route. Apply saves/activates,
Cancel discards, Back/Escape with edits offers Apply/Discard/Stay. Derive
all instructions/cues from bindings and behavior modes.

Implement timed tap removal: same Material duration, one original target,
one completion, second tap cancels. Lost target/reach/valid aim or new place,
sample, hand/slot/Shape/orientation/history actions cancel without resuming.
Toggle sprint persists over stops but clears on menu/focus loss/respawn.
Show its indicator; hold behaviors remain defaults.

## Acceptance criteria

- [x] Capture/settings navigation sends zero unintended gameplay intents.
- [x] Tap never bypasses timing, transfers progress or repeats automatically.
- [x] Toggle sprint resets as specified; modes and labels survive restart.
- [x] Test non-US layout, both modifier sides and default redo alternatives.
- [x] Retain native settings/gesture captures for visual review.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices and shared understanding are confirmed. Local
implementation is authorized; push, publication and merge are not.

## Comments

### 2026-10-27: repository-based triage

Draft scope/acceptance require grilling; do not begin implementation.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Controls scope accepted during grilling

The user accepted sprint toggle, timed tap-removal mode, mouse sensitivity,
invert Y and hotbar scroll preferences alongside remapping. Save/reset these
in the user profile and expose them in controls settings. Preserve current
hold defaults and Game removal duration. See spec.md and grilling.md for
settled behavior and the remaining decisions; readiness is unchanged.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.

### 2026-10-28: Agent Brief

Implement as part of the confirmed combined PR 2, preserving the acceptance
criteria above. Earlier readiness and separate-PR statements are superseded.

### 2026-10-28: implementation complete

Implemented and validated in the sequential local PR 2 batches. Evidence,
artifact repair and retained native captures are recorded in
[PR 2 validation](../pr2-implementation-validation.md). Visual fixtures are
separate from timing/intent regressions. No push, publication or merge.
