# 03: C1: User controls and behavior profile

Status: needs-triage
Blocked by: 02

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C1; keep this separate from other PR scopes.

## What to build

Persist stable action/key ids, two keyboard/mouse alternatives per action,
context-aware conflict rules and behavior preferences in one Engine user
profile. Include hold/toggle sprint, hold/timed-tap removal, mouse sensitivity,
invert Y and hotbar scroll preferences. Preserve current defaults. Use atomic
writes, reset, schema/default merging and recoverable corrupt data handling.
Keep this separate from Worlds, per-Game shortcuts and owned inventory.

## Acceptance criteria

- [ ] Restart/reset/corrupt-profile/new-action cases preserve valid preferences.
- [ ] Binding conflicts offer replace/cancel in overlapping contexts only.
- [ ] Labels are layout-aware and independent of serialized physical ids.
- [ ] Profile writes contain no runtime Material ids or World state.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

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
