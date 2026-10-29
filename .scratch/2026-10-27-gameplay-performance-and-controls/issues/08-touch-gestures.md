# 08: C4: Touch pointer ownership and actions

Status: needs-triage
Blocked by: 02, 06

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C4; keep this separate from other PR scopes.

## What to build

Give each native desktop touch id a role at start: move, look, control or UI.
Retain ownership through end/cancel, supporting concurrent contacts without
transferring gestures when pointers disappear. Cancel on focus/menu changes.
Suppress duplicate synthetic mouse actions including final release, while
allowing subsequent genuine mouse input. Route through semantic actions and
Server intents, never direct World access or writes to raw mouse input.

## Acceptance criteria

- [ ] Reordered move/look contacts preserve independent axes and identity.
- [ ] Unknown/replaced ids cannot inherit removal progress.
- [ ] UI scroll/close/end/cancel cannot become a gameplay Edit.
- [ ] Touch plus synthetic mouse yields at most one intended action.

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

### World screens added by ticket 27 (2026-10-29)

Ticket 27 ships the World list, Create, Rename, Delete and Trust dialogs for
mouse and keyboard only, with Tab handling local to those screens. This
ticket's journeys must cover them; ticket 27 no longer claims that support.
