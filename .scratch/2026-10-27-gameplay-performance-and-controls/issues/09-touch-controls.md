# 09: C4: Touch overlay and preferences

Status: needs-triage
Blocked by: 08, 04

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C4; keep this separate from other PR scopes.

## What to build

Intentional touch reveals optional left-move/right-look controls and separate
place/remove actions. Include access to vertical movement, slots, Shape/
orientation, empty hand, sample, Use, view/mode and catalogue operations as
those capabilities become available. Preserve tap/hold cancellation rules.
Offer normal/mirrored presets, size and opacity; no freeform HUD editor.
Mouse play does not retain a permanent overlay. Keep keyboard/mouse usable,
44-pixel targets spaced and the Server target/progress cues unobscured.

## Acceptance criteria

- [ ] Play and catalogue operations require no hover or keyboard numbers.
- [ ] Gestures never leak through overlay/UI transitions.
- [ ] Preset/size/opacity settings respect Apply/Cancel and reset behavior.
- [ ] Retain normal/small/enlarged native overlay captures for review.

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

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
