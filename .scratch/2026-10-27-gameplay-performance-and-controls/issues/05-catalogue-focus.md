# 05: C2: Focus, selection and explicit assignment

Status: needs-triage
Blocked by: 02

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C2; keep this separate from other PR scopes.

## What to build

Add stable keyboard/controller focus and visible Hold/Assign to slot
operations alongside existing mouse quick-hold, drag/drop and number keys.
Explicit Material selection owns the Shape panel; passing the mouse over
another tile cannot replace it. Hover still highlights/previews; focus
selects the corresponding panel. Preserve source entities during drag.
Restore focus from dialogs; expose reusable operations for touch/controller.

## Acceptance criteria

- [ ] Choose Shape, hold, assign, swap and cancel without hover/drag.
- [ ] Mouse shortcuts remain compatible and never click through to World edits.
- [ ] Focus/selection and Shape panel stay stable through rebuilds.
- [ ] Focus indicators and invalid feedback do not rely only on colour.

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
